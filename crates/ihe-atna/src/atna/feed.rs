// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The ATX: FHIR Feed Option of ITI-20 (feature `smart`).
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
//! The repository is reached through `ferrohealth-smart`'s
//! [`OutboundClient`], the one outbound client of the FerroHEALTH products,
//! built from the repository's endpoint table. It carries the TLS, the
//! timeouts, the authorization (HTTP Basic, or a client credentials token
//! renewed once on a `401`, as RFC 6750 §3.1 has a client do on
//! `invalid_token`) and the endpoint's `retry` table. One send is one round
//! of that table's tries; the [`Forwarder`](super::forwarder::Forwarder)
//! waits the table's `max_backoff` between rounds and never gives a record
//! up.
//!
//! The record names a patient, so the repository is reached over TLS unless
//! the caller accepts clear text by name ([`FeedRepository::cleartext`]), the
//! client follows no redirect, and no error carries the record, the
//! credential or the repository's answer body.
//!
//! # Examples
//!
//! A BALP record of a search the system made, stored in a spool of
//! `AuditEvent`s and sent over the FHIR Feed.
//!
//! ```no_run
//! use std::path::Path;
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use ferrohealth_smart::client::outbound::OutboundClient;
//! use ferrohealth_smart::config::TlsBuilder;
//! use ferrohealth_smart::config::endpoint::EndpointSection;
//! use ihe_atna::atna::feed::FeedRepository;
//! use ihe_atna::atna::forwarder::Forwarder;
//! use ihe_atna::atna::spool::{Bounds, Content, Spool};
//! use ihe_atna::balp::{
//!     DESTINATION_ROLE, Direction, EventKind, Exchange, NetworkAddress, Observer, Outcome,
//!     Peer, REST, SEARCH, SOURCE_APPLICATION_SERVER, SOURCE_ROLE,
//! };
//! use ihe_atna::user::OnBehalfOf;
//! use url::Url;
//!
//! # fn configured() -> (EndpointSection, &'static dyn TlsBuilder) {
//! #     unimplemented!("the product's configuration and its reader of TLS material")
//! # }
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // The `[audit.fhir_feed.endpoint]` table and the product's TLS reader.
//! let (endpoint, tls) = configured();
//! let client = OutboundClient::build(&endpoint, "audit.fhir_feed.endpoint", tls)?;
//! let feed = FeedRepository::new(client)?;
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

use ferrohealth_smart::client::outbound::{OutboundClient, SendError};
use ferrohealth_smart::config::endpoint::RetrySection;
use http::header::CONTENT_TYPE;
use http::{Method, StatusCode};
use secrecy::{ExposeSecret, SecretSlice};
use url::Url;

use super::retry::{Retry, RetryError};
use crate::redact::RedactedUrl;

/// The media type a record is sent as (ITI TF-2 Appendix Z.6).
const FHIR_JSON: &str = "application/fhir+json";

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
    /// The endpoint's `retry` table gives the forwarder no wait between two
    /// rounds: its `max_backoff` is zero.
    #[error("the audit repository endpoint's retry table is refused")]
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
    /// The repository answered `401`, after a fresh token when the endpoint
    /// names a client credentials grant: the credential is at fault, never
    /// the record, which stays stored.
    #[error("the audit repository did not accept the credential")]
    Unauthorized,
    /// No token could be obtained for the repository, or a refused one
    /// renewed.
    #[error("no credential for the audit repository could be had")]
    Credentials(#[source] SendError),
    /// The repository answered a status that may pass: a `5xx`, `408`,
    /// `429`, or a status the create interaction gives no meaning to.
    #[error("the audit repository answered {status}")]
    Unavailable {
        /// The HTTP status.
        status: StatusCode,
    },
    /// The request could not be sent, or no answer arrived within the
    /// timeout, on the last try of the round.
    #[error("the audit repository could not be reached")]
    Transport(#[source] reqwest::Error),
}

/// An Audit Record Repository's FHIR Feed endpoint, reached through the
/// [`OutboundClient`] its endpoint table builds.
///
/// `Debug` shows the endpoint without its userinfo or query and whether the
/// client carries an authorization, never the client itself.
#[derive(Clone)]
pub struct FeedRepository {
    endpoint: Url,
    client: OutboundClient,
    between_rounds: Retry,
}

impl FeedRepository {
    /// The repository whose FHIR base is `client`'s URL, reached over
    /// `https`.
    ///
    /// # Errors
    ///
    /// [`FeedConfigError::Base`] for a URL that is no `http(s)` URL without a
    /// query or a fragment, [`FeedConfigError::Cleartext`] for an `http`
    /// one, and [`FeedConfigError::Retry`] for a zero
    /// `retry.max_backoff`.
    pub fn new(client: OutboundClient) -> Result<Self, FeedConfigError> {
        match client.url().scheme() {
            "https" => Self::any_scheme(client),
            "http" => Err(FeedConfigError::Cleartext),
            _ => Err(FeedConfigError::Base),
        }
    }

    /// The repository at `client`'s URL over `http` or `https`: the caller
    /// accepts that its records, which name patients, may cross the network
    /// in clear text, such as to a TLS terminator beside it that the operator
    /// answers for.
    ///
    /// # Errors
    ///
    /// [`FeedConfigError::Base`] for a URL that is no `http(s)` URL without a
    /// query or a fragment, and [`FeedConfigError::Retry`] for a zero
    /// `retry.max_backoff`.
    pub fn cleartext(client: OutboundClient) -> Result<Self, FeedConfigError> {
        Self::any_scheme(client)
    }

    fn any_scheme(client: OutboundClient) -> Result<Self, FeedConfigError> {
        let endpoint = crate::fhir_base::under_base(client.url().clone(), "AuditEvent")
            .ok_or(FeedConfigError::Base)?;
        let between_rounds = between_rounds(client.retry())?;
        Ok(Self {
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
    /// every failed send waits the endpoint's `max_backoff`.
    pub(crate) fn between_rounds(&self) -> Retry {
        self.between_rounds
    }

    /// Posts `record`, one `AuditEvent` as FHIR JSON (§3.20.4.2.2), in one
    /// round of the endpoint's tries.
    ///
    /// # Errors
    ///
    /// A [`FeedError`] for every last answer but a `2xx`, and for no answer.
    pub async fn send(&self, record: &SecretSlice<u8>) -> Result<(), FeedError> {
        let answer = self
            .client
            .send(&Method::POST, &self.endpoint, |request| {
                request
                    .header(CONTENT_TYPE, FHIR_JSON)
                    .body(record.expose_secret().to_vec())
            })
            .await
            .map_err(|error| match error {
                SendError::Transport(error) => FeedError::Transport(error.without_url()),
                other => FeedError::Credentials(other),
            })?;
        outcome(answer.status())
    }
}

/// One attempt a round, each failed round followed by `max_backoff`: the
/// round's own tries are [`OutboundClient::send`]'s.
fn between_rounds(retry: &RetrySection) -> Result<Retry, RetryError> {
    let wait = std::time::Duration::from(retry.max_backoff);
    Retry::new(1, wait, wait)
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
            .field("authorization", &self.client.authorization().is_some())
            .finish_non_exhaustive()
    }
}
