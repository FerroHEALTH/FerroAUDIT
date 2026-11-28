// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The HTTP routes the gate loads, each handing its request to `ihe-atna`
//! as a product's server does.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract;
use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::Response;
use axum::routing::{get, post};
use ihe_atna::arr::feed::{Answer, Format, receive};
use ihe_atna::arr::search::{Retrieval, retrieve};
use ihe_atna::arr::{AuditStore, Requester};
use ihe_atna::atna::codes;
use ihe_atna::atna::forwarder::Forwarder;
use ihe_atna::atna::message::{
    ActiveParticipant, AuditMessage, AuditSource, CodedValue, EventIdentification,
    ParticipantObject,
};
use ihe_atna::atna::syslog::Sender;
use secrecy::SecretString;

use crate::store::Store;

/// What every route shares.
#[derive(Debug)]
pub(crate) struct State {
    /// The repository's store.
    pub(crate) store: Arc<Store>,
    /// The ITI-20 sender's forwarder.
    pub(crate) sending: Arc<Forwarder>,
    /// The ITI-20 sender's syslog header.
    pub(crate) sender: Sender,
    /// The absolute URL of the `AuditEvent` search.
    pub(crate) endpoint: String,
}

type Shared = extract::State<Arc<State>>;

/// The routes: the FHIR Feed `batch` (`POST /fhir`) and `create` (`POST
/// /fhir/AuditEvent`), ITI-81 (`GET /fhir/AuditEvent`), the ITI-20 send
/// (`POST /syslog`) and the readiness probe (`GET /health`).
pub(crate) fn router(state: Arc<State>) -> Router {
    Router::new()
        .route("/fhir", post(feed))
        .route("/fhir/AuditEvent", post(feed).get(search))
        .route("/syslog", post(syslog))
        .route("/health", get(|| async { StatusCode::OK }))
        .with_state(state)
}

/// ITI TF-2 §3.20.4.2 and §3.20.4.4: a `create` or a `batch`.
async fn feed(extract::State(state): Shared, headers: HeaderMap, body: Bytes) -> Response {
    let Some(format) = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(Format::of)
    else {
        return status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    };
    let store: &dyn AuditStore = state.store.as_ref();
    answer(receive(store, format, &body).await)
}

/// ITI TF-2 §3.81.4: a retrieval.
async fn search(extract::State(state): Shared, uri: Uri) -> Response {
    let requester = Requester {
        user: None,
        address: Some("127.0.0.1".to_owned()),
    };
    let store: &dyn AuditStore = state.store.as_ref();
    let retrieval = Retrieval {
        query: uri.query().unwrap_or_default(),
        endpoint: &state.endpoint,
        requester: &requester,
        observer: "perf.example.org",
    };
    answer(retrieve(store, retrieval).await)
}

/// ITI TF-2 §3.20.4.1: the DICOM audit message of a read of the patient
/// the body names, framed and stored for the sender's forwarder. The
/// answer is `202` once the spool stored it, which is what an audited
/// request waits for.
async fn syslog(extract::State(state): Shared, body: Bytes) -> Response {
    let Ok(patient) = String::from_utf8(body.to_vec()) else {
        return status(StatusCode::BAD_REQUEST);
    };
    let message = AuditMessage {
        event: EventIdentification {
            event_id: CodedValue::of(codes::EVENT_PATIENT_RECORD),
            action: "R",
            date_time: jiff::Timestamp::now(),
            outcome: "0",
            type_codes: Vec::new(),
            outcome_description: None,
            purposes_of_use: Vec::new(),
        },
        participants: vec![ActiveParticipant {
            user_id: "Qz7-user".to_owned(),
            alternative_user_id: None,
            user_name: None,
            user_is_requestor: true,
            role_id_codes: vec![CodedValue::of(codes::ROLE_SOURCE_ROLE_ID)],
            access_point: None,
        }],
        source: AuditSource {
            id: "perf.example.org".to_owned(),
            enterprise_site: None,
            type_codes: Vec::new(),
        },
        objects: vec![ParticipantObject {
            id: SecretString::from(patient),
            type_code: Some("1"),
            type_code_role: Some("1"),
            data_life_cycle: None,
            id_type_code: CodedValue::of(codes::OBJECT_ID_PATIENT_NUMBER),
            name: None,
            query: None,
            details: Vec::new(),
        }],
    };
    let Ok(xml) = message.to_xml() else {
        return status(StatusCode::UNPROCESSABLE_ENTITY);
    };
    let frame = state.sender.frame(jiff::Timestamp::now(), &xml);
    match state.sending.submit(frame).await {
        Ok(()) => status(StatusCode::ACCEPTED),
        Err(_full_or_unwritable) => status(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// The HTTP response of a repository answer, its body in JSON.
fn answer(answer: Answer) -> Response {
    let mut response = Response::builder().status(answer.status);
    if let Some(location) = answer.location {
        response = response.header(header::LOCATION, location);
    }
    let body = match answer.body.map(|resource| serde_json::to_vec(&resource)) {
        Some(Ok(json)) => {
            response = response.header(header::CONTENT_TYPE, "application/fhir+json");
            Body::from(json)
        }
        Some(Err(_unwritable)) => return status(StatusCode::INTERNAL_SERVER_ERROR),
        None => Body::empty(),
    };
    response
        .body(body)
        .unwrap_or_else(|_unbuildable| status(StatusCode::INTERNAL_SERVER_ERROR))
}

/// An empty response of `code`.
fn status(code: StatusCode) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = code;
    response
}
