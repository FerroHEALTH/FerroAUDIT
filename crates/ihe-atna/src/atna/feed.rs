// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The ATX: FHIR Feed Option of ITI-20 (feature `feed`).
//!
//! Send Audit Resource Request is a FHIR `create` of one `AuditEvent` at an
//! Audit Record Repository (the `RESTful` ATNA supplement, ITI TF-2
//! §3.20.4.2), answered by Send Audit Resource Response (§3.20.4.3).
//!
//! [`FeedRepository::send`] posts one stored record to `[base]/AuditEvent` as
//! FHIR JSON (ITI TF-2 Appendix Z.6; FHIR R4 `create`,
//! <http://hl7.org/fhir/R4/http.html#create>). A `2xx` is the repository's
//! success (§3.20.4.3.2). §3.20.4.3.3 leaves a failure to the client: a
//! `4xx` other than `401`, `408` and `429` is [`FeedError::Rejected`], which
//! no retry of the same record changes; a `401` is
//! [`FeedError::Unauthorized`], a fault of the credential and never of the
//! record; and every other failure is [`FeedError::Unavailable`], which a
//! later attempt may get past.
//!
//! The request goes out through the caller's [`FeedClient`], so the library
//! depends on no HTTP client and no auth library. The caller's client owns
//! the TLS, the timeouts, the credential and its renewal (a client
//! credentials token renewed once on a `401`, as RFC 6750 §3.1 has a client
//! do on `invalid_token`) and the tries of one round. The
//! [`Forwarder`](super::forwarder::Forwarder) waits the repository's
//! `between_rounds` after each failed send and never gives a record up. The
//! seam is our own design: no specification governs it.
//!
//! The record names a patient, so the repository is reached over TLS unless
//! the caller accepts clear text by name ([`FeedRepository::cleartext`]). The
//! caller's client follows no redirect, and no error the library raises
//! carries the record, the credential or the repository's answer body.
//!
//! # Examples
//!
//! A product's client over `ferrohealth-smart`'s `OutboundClient`, and a
//! BALP record of a search the system made, stored in a spool of
//! `AuditEvent`s and sent over the FHIR Feed.
//!
//! ```no_run
//! use std::path::Path;
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use ferrohealth_smart::client::outbound::{OutboundClient, SendError};
//! use ferrohealth_smart::config::TlsBuilder;
//! use ferrohealth_smart::config::endpoint::EndpointSection;
//! use http::{Method, header::CONTENT_TYPE};
//! use ihe_atna::atna::feed::{FeedAnswer, FeedClient, FeedRepository, FeedSendError};
//! use ihe_atna::atna::forwarder::Forwarder;
//! use ihe_atna::atna::spool::{Bounds, Content, Spool};
//! use ihe_atna::balp::{
//!     DESTINATION_ROLE, Direction, EventKind, Exchange, NetworkAddress, Observer, Outcome,
//!     Peer, REST, SEARCH, SOURCE_APPLICATION_SERVER, SOURCE_ROLE,
//! };
//! use ihe_atna::user::OnBehalfOf;
//! use url::Url;
//!
//! /// The product's own outbound client, as the FHIR Feed's client.
//! struct Smart(OutboundClient);
//!
//! #[async_trait::async_trait]
//! impl FeedClient for Smart {
//!     fn authorizes(&self) -> bool {
//!         self.0.authorization().is_some()
//!     }
//!
//!     async fn post(
//!         &self,
//!         url: &Url,
//!         content_type: &'static str,
//!         body: &[u8],
//!     ) -> Result<FeedAnswer, FeedSendError> {
//!         let answer = self
//!             .0
//!             .send(&Method::POST, url, |request| {
//!                 request.header(CONTENT_TYPE, content_type).body(body.to_vec())
//!             })
//!             .await
//!             .map_err(|error| match error {
//!                 SendError::Transport(error) => FeedSendError::Transport(Box::new(error.without_url())),
//!                 other => FeedSendError::Credentials(Box::new(other)),
//!             })?;
//!         let status = answer.status();
//!         let body = answer
//!             .bytes()
//!             .await
//!             .map_err(|error| FeedSendError::Transport(Box::new(error.without_url())))?;
//!         Ok(FeedAnswer::new(status, body.to_vec()))
//!     }
//! }
//!
//! # fn configured() -> (EndpointSection, &'static dyn TlsBuilder) {
//! #     unimplemented!("the product's configuration and its reader of TLS material")
//! # }
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // The `[audit.fhir_feed.endpoint]` table and the product's TLS reader.
//! let (endpoint, tls) = configured();
//! let client = OutboundClient::build(&endpoint, "audit.fhir_feed.endpoint", tls)?;
//! let base = client.url().clone();
//! let between_rounds = Duration::from(client.retry().max_backoff);
//! let feed = FeedRepository::new(&base, Arc::new(Smart(client)), between_rounds)?;
//! let bounds = Bounds {
//!     max_messages: 10_000,
//!     max_bytes: 64 << 20,
//!     write_timeout: Duration::from_secs(2),
//! };
//! let spool = Spool::open_for(Path::new("/var/spool/audit-feed"), bounds, Content::AuditEvents)?;
//! let forwarder = Forwarder::fhir_feed(spool, feed);
//! tokio::spawn(Arc::clone(&forwarder).run());
//!
//! let exchange = Exchange {
//!     kind: EventKind {
//!         profile: "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Query",
//!         event_type: REST,
//!         subtypes: vec![SEARCH],
//!         action: "E",
//!         client: SOURCE_ROLE,
//!         server: DESTINATION_ROLE,
//!     },
//!     recorded: jiff::Timestamp::now(),
//!     outcome: Outcome::Success,
//!     outcome_description: None,
//!     direction: Direction::Sent {
//!         server: Peer::server(&Url::parse("https://directory.example.org/fhir/")?),
//!     },
//!     on_behalf: OnBehalfOf::System,
//!     entities: Vec::new(),
//!     security: Vec::new(),
//! };
//! let observer = Observer {
//!     source_id: "gateway.example.org".to_owned(),
//!     site: None,
//!     host: NetworkAddress::host("gateway.example.org"),
//!     source_types: vec![SOURCE_APPLICATION_SERVER],
//! };
//! forwarder
//!     .submit(exchange.audit_event(&observer)?.into_bytes())
//!     .await?;
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use http::StatusCode;
use secrecy::{ExposeSecret, SecretSlice};
use url::Url;

use super::retry::{Retry, RetryError};
use crate::redact::RedactedUrl;

/// The media type a record is sent as (ITI TF-2 Appendix Z.6).
const FHIR_JSON: &str = "application/fhir+json";

/// The caller's client a [`FeedRepository`] sends through.
///
/// It owns the TLS, the timeouts, the credential and its renewal, and the
/// tries of one round; it follows no redirect, and an error it returns
/// carries no record, credential or answer body. Our own design: no
/// specification governs the seam.
#[async_trait::async_trait]
pub trait FeedClient: Send + Sync {
    /// Whether the client sends a credential, which the repository's `Debug`
    /// shows without its value.
    fn authorizes(&self) -> bool;

    /// Posts `body` as `content_type` to `url` with the repository's
    /// credential, in one round of the client's tries, and returns the last
    /// answer's status and body.
    ///
    /// # Errors
    ///
    /// [`FeedSendError::Credentials`] when no credential could be had, and
    /// [`FeedSendError::Transport`] when no answer, or no whole body,
    /// arrived.
    async fn post(
        &self,
        url: &Url,
        content_type: &'static str,
        body: &[u8],
    ) -> Result<FeedAnswer, FeedSendError>;
}

/// A repository's answer: its status and its body. `Debug` shows the status
/// and the body's length, never the body.
pub struct FeedAnswer {
    status: StatusCode,
    body: Vec<u8>,
}

impl FeedAnswer {
    /// The answer of `status` with `body`.
    #[must_use]
    pub fn new(status: StatusCode, body: Vec<u8>) -> Self {
        Self { status, body }
    }

    /// The status.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }
}

impl fmt::Debug for FeedAnswer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FeedAnswer")
            .field("status", &self.status)
            .field("body", &self.body.len())
            .finish()
    }
}

/// Why a [`FeedClient`] sent no request or had no answer.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FeedSendError {
    /// No credential could be obtained for the repository, or a refused one
    /// renewed.
    #[error("no credential for the audit repository could be had")]
    Credentials(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The request could not be sent, or no answer arrived within the
    /// timeout, on the last try of the round.
    #[error("the audit repository could not be reached")]
    Transport(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Why a repository's endpoint was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FeedConfigError {
    /// The endpoint URL is not an `https` URL, or an `http` one for
    /// development, without a query or a fragment.
    #[error("the audit repository FHIR base is not an https URL without a query or a fragment")]
    Base,
    /// The endpoint URL is `http` and the caller did not accept clear text.
    #[error("the audit repository FHIR base is not https")]
    Cleartext,
    /// The wait between two rounds is zero, which gives the forwarder no
    /// wait between them.
    #[error("the audit repository's wait between rounds is refused")]
    Retry(#[from] RetryError),
}

/// Why a record was not accepted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FeedError {
    /// The repository refused the record with a `4xx` other than `401`,
    /// `408` or `429`: sending it again would be refused again.
    #[error("the audit repository refused the AuditEvent with {status}")]
    Rejected {
        /// The HTTP status.
        status: StatusCode,
    },
    /// The repository answered `401`, after a fresh token when the caller's
    /// client renews one: the credential is at fault, never the record,
    /// which stays stored.
    #[error("the audit repository did not accept the credential")]
    Unauthorized,
    /// The caller's client sent nothing or had no answer.
    #[error("the audit repository could not be sent to")]
    Send(#[source] FeedSendError),
    /// The repository answered a status that may pass: a `5xx`, `408`,
    /// `429`, or a status the create interaction gives no meaning to.
    #[error("the audit repository answered {status}")]
    Unavailable {
        /// The HTTP status.
        status: StatusCode,
    },
}

/// An Audit Record Repository's FHIR Feed endpoint, reached through the
/// caller's [`FeedClient`].
///
/// `Debug` shows the endpoint without its userinfo or query and whether the
/// client sends a credential, never the client itself.
#[derive(Clone)]
pub struct FeedRepository {
    base: Url,
    endpoint: Url,
    client: Arc<dyn FeedClient>,
    between_rounds: Retry,
}

impl FeedRepository {
    /// The repository whose FHIR base is `base`, reached over `https`
    /// through `client`, whose forwarder waits `between_rounds` after each
    /// failed send.
    ///
    /// # Errors
    ///
    /// [`FeedConfigError::Base`] for a URL that is no `http(s)` URL without a
    /// query or a fragment, [`FeedConfigError::Cleartext`] for an `http`
    /// one, and [`FeedConfigError::Retry`] for a zero `between_rounds`.
    pub fn new(
        base: &Url,
        client: Arc<dyn FeedClient>,
        between_rounds: Duration,
    ) -> Result<Self, FeedConfigError> {
        match base.scheme() {
            "https" => Self::any_scheme(base, client, between_rounds),
            "http" => Err(FeedConfigError::Cleartext),
            _ => Err(FeedConfigError::Base),
        }
    }

    /// The repository at `base` over `http` or `https`: the caller accepts
    /// that its records, which name patients, may cross the network in clear
    /// text, such as to a TLS terminator beside it that the operator answers
    /// for.
    ///
    /// # Errors
    ///
    /// [`FeedConfigError::Base`] for a URL that is no `http(s)` URL without a
    /// query or a fragment, and [`FeedConfigError::Retry`] for a zero
    /// `between_rounds`.
    pub fn cleartext(
        base: &Url,
        client: Arc<dyn FeedClient>,
        between_rounds: Duration,
    ) -> Result<Self, FeedConfigError> {
        Self::any_scheme(base, client, between_rounds)
    }

    fn any_scheme(
        base: &Url,
        client: Arc<dyn FeedClient>,
        between_rounds: Duration,
    ) -> Result<Self, FeedConfigError> {
        let endpoint = crate::fhir_base::under_base(base.clone(), "AuditEvent")
            .ok_or(FeedConfigError::Base)?;
        // NOTE: one attempt a round, each failed round followed by the wait: the
        // round's own tries are the caller's client's.
        let between_rounds = Retry::new(1, between_rounds, between_rounds)?;
        Ok(Self {
            base: base.clone(),
            endpoint,
            client,
            between_rounds,
        })
    }

    /// The `[base]/AuditEvent` URL records are posted to.
    #[must_use]
    pub fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    /// The forwarder's policy for this repository: one send is a round, so
    /// every failed send waits `between_rounds`.
    pub(crate) fn between_rounds(&self) -> Retry {
        self.between_rounds
    }

    /// Posts `record`, one `AuditEvent` as FHIR JSON (§3.20.4.2.2), in one
    /// round of the client's tries.
    ///
    /// # Errors
    ///
    /// A [`FeedError`] for every last answer but a `2xx`, and for no answer.
    pub async fn send(&self, record: &SecretSlice<u8>) -> Result<(), FeedError> {
        let answer = self
            .client
            .post(&self.endpoint, FHIR_JSON, record.expose_secret())
            .await
            .map_err(FeedError::Send)?;
        outcome(answer.status)
    }

    /// Posts `records`, each one `AuditEvent` as FHIR JSON, as one Bundle of
    /// type `batch` to the FHIR base (ITI TF-2 §3.20.4.4.2), in one round of
    /// the client's tries, and returns each record's outcome from its entry
    /// of the `batch-response` (§3.20.4.5.2), in order.
    ///
    /// # Errors
    ///
    /// A [`FeedError`] when the Bundle as a whole is not accepted: no answer,
    /// a status that is no `2xx`, or a body that is no `batch-response` with
    /// one entry per record. [`FeedError::Rejected`] then names a refusal of
    /// the Bundle itself, which a caller can follow by sending each record on
    /// its own.
    pub async fn send_batch(
        &self,
        records: &[&SecretSlice<u8>],
    ) -> Result<Vec<Result<(), FeedError>>, FeedError> {
        let mut body = br#"{"resourceType":"Bundle","type":"batch","entry":["#.to_vec();
        for (index, record) in records.iter().enumerate() {
            if index > 0 {
                body.push(b',');
            }
            body.extend_from_slice(br#"{"resource":"#);
            body.extend_from_slice(record.expose_secret());
            body.extend_from_slice(br#","request":{"method":"POST","url":"AuditEvent"}}"#);
        }
        body.extend_from_slice(b"]}");
        let body = SecretSlice::from(body);
        let answer = self
            .client
            .post(&self.base, FHIR_JSON, body.expose_secret())
            .await
            .map_err(FeedError::Send)?;
        outcome(answer.status)?;
        let entries = batch_response(&answer.body).ok_or(FeedError::Unavailable {
            status: answer.status,
        })?;
        if entries.len() != records.len() {
            return Err(FeedError::Unavailable {
                status: answer.status,
            });
        }
        Ok(entries.into_iter().map(outcome).collect())
    }
}

/// The status of each entry of a `batch-response` Bundle (ITI TF-2
/// §3.20.4.5.2: each entry's response carries the HTTP status of its
/// request), or `None` for a body that is no such Bundle.
fn batch_response(body: &[u8]) -> Option<Vec<StatusCode>> {
    // NOTE: no specification governs this: our own design; a body that is no Bundle is
    // answered by the caller as a repository that did not take the batch, never dropped.
    let bundle: fhir_types::r4::bundle::Bundle = serde_json::from_slice(body).ok()?;
    if bundle.r#type.value.as_deref() != Some("batch-response") {
        return None;
    }
    bundle
        .entry
        .iter()
        .map(|entry| {
            let status = entry.response.as_ref()?.status.value.as_deref()?;
            // NOTE: FHIR R4 Bundle.entry.response.status starts with the 3-digit HTTP code.
            let code = status.get(..3)?;
            StatusCode::from_bytes(code.as_bytes()).ok()
        })
        .collect()
}

/// What the repository's answer `status` means for the record.
fn outcome(status: StatusCode) -> Result<(), FeedError> {
    if status.is_success() {
        return Ok(());
    }
    if status == StatusCode::UNAUTHORIZED {
        return Err(FeedError::Unauthorized);
    }
    if status.is_client_error()
        && status != StatusCode::REQUEST_TIMEOUT
        && status != StatusCode::TOO_MANY_REQUESTS
    {
        return Err(FeedError::Rejected { status });
    }
    Err(FeedError::Unavailable { status })
}

impl fmt::Debug for FeedRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FeedRepository")
            .field("endpoint", &RedactedUrl(self.endpoint.as_str()))
            .field("authorization", &self.client.authorizes())
            .finish_non_exhaustive()
    }
}
