// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The ATX: FHIR Feed Option of ITI-20 (feature `balp`).
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
//! A repository that asks for a bearer token takes a [`Credentials`] source,
//! asked for the `Authorization` value of each request and told of a `401`,
//! after which the request is sent once more with a fresh value, as RFC 6750
//! §3.1 has a client do on `invalid_token`.
//!
//! The record names a patient, so the repository is reached over TLS unless
//! the caller accepts clear text by name ([`FeedRepository::cleartext`]), no
//! redirect is followed, and no error carries the record, the credential or
//! the repository's answer body. The TLS stack is the caller's: it builds the
//! `reqwest::Client` with the TLS backend it ships.
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
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let http = reqwest::Client::builder()
//!     .redirect(reqwest::redirect::Policy::none())
//!     .build()?;
//! let feed = FeedRepository::new(
//!     Url::parse("https://arr.example.org/fhir/")?,
//!     http,
//!     Duration::from_secs(10),
//! )?;
//! let bounds = Bounds {
//!     max_messages: 10_000,
//!     max_bytes: 64 << 20,
//!     write_timeout: Duration::from_secs(2),
//! };
//! let spool = Spool::open_for(Path::new("/var/spool/audit-feed"), bounds, Content::AuditEvents)?;
//! let forwarder = Forwarder::fhir_feed(spool, feed, Duration::from_secs(60));
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
use std::time::Duration;

use std::sync::Arc;

use http::StatusCode;
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretSlice, SecretString};
use url::Url;

use crate::redact::RedactedUrl;

/// The media type a record is sent as (ITI TF-2 Appendix Z.6).
const FHIR_JSON: &str = "application/fhir+json";

/// Why a repository's FHIR base was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FeedAddressError {
    /// The base is not an `https` URL, or an `http` one for development,
    /// without a query or a fragment.
    #[error("the audit repository FHIR base is not an https URL without a query or a fragment")]
    Base,
    /// The base is `http` and the caller did not accept clear text.
    #[error("the audit repository FHIR base is not https")]
    Cleartext,
}

/// Where the `Authorization` value of each Feed request comes from, such as a
/// cache of OAuth 2.0 access tokens a client renews.
#[async_trait::async_trait]
pub trait Credentials: Send + Sync {
    /// The `Authorization` header value of the next request, such as
    /// `Bearer <token>`.
    ///
    /// # Errors
    ///
    /// A [`CredentialsError`] when no value can be had now; the record stays
    /// stored and is sent again later.
    async fn authorization(&self) -> Result<SecretString, CredentialsError>;

    /// Told that the repository answered `401` to `refused`, so a value it
    /// caches is renewed before the next [`Credentials::authorization`].
    async fn refused(&self, refused: &SecretString);
}

/// Why a [`Credentials`] source gave no value.
#[derive(Debug, thiserror::Error)]
#[error("no credential for the audit repository could be had")]
pub struct CredentialsError(#[source] pub Box<dyn std::error::Error + Send + Sync>);

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
    /// The repository answered `401`, after a fresh credential when the
    /// repository has a [`Credentials`] source: the credential is at fault,
    /// never the record, which stays stored.
    #[error("the audit repository did not accept the credential")]
    Unauthorized,
    /// The [`Credentials`] source gave no value.
    #[error("no credential for the audit repository could be had")]
    Credentials(#[source] CredentialsError),
    /// The repository answered a status that may pass: a `5xx`, `408`,
    /// `429`, or a status the create interaction gives no meaning to.
    #[error("the audit repository answered {status}")]
    Unavailable {
        /// The HTTP status.
        status: StatusCode,
    },
    /// The request could not be sent, or no answer arrived within the
    /// timeout.
    #[error("the audit repository could not be reached")]
    Transport(#[source] reqwest::Error),
}

/// An Audit Record Repository's FHIR Feed endpoint.
///
/// `Debug` shows the endpoint without its userinfo and leaves out the HTTP
/// client, whose default headers may hold a credential.
#[derive(Clone)]
pub struct FeedRepository {
    endpoint: Url,
    http: reqwest::Client,
    timeout: Duration,
    credentials: Option<Arc<dyn Credentials>>,
}

impl FeedRepository {
    /// The repository whose FHIR base is `base`, reached over `https`
    /// through `http`, each request bounded by `timeout`.
    ///
    /// Build `http` with `redirect::Policy::none()`: a record names a
    /// patient, and a client that follows redirects sends it wherever the
    /// repository points.
    ///
    /// # Errors
    ///
    /// [`FeedAddressError::Base`] for a base that is no `http(s)` URL
    /// without a query or a fragment, and [`FeedAddressError::Cleartext`]
    /// for an `http` one.
    pub fn new(
        base: Url,
        http: reqwest::Client,
        timeout: Duration,
    ) -> Result<Self, FeedAddressError> {
        if base.scheme() != "https" {
            return if base.scheme() == "http" {
                Err(FeedAddressError::Cleartext)
            } else {
                Err(FeedAddressError::Base)
            };
        }
        Self::any_scheme(base, http, timeout)
    }

    /// The repository at `base` over `http` or `https`: the caller accepts
    /// that its records, which name patients, may cross the network in
    /// clear text, such as to a TLS terminator beside it that the operator
    /// answers for.
    ///
    /// # Errors
    ///
    /// [`FeedAddressError::Base`] for a base that is no `http(s)` URL
    /// without a query or a fragment.
    pub fn cleartext(
        base: Url,
        http: reqwest::Client,
        timeout: Duration,
    ) -> Result<Self, FeedAddressError> {
        Self::any_scheme(base, http, timeout)
    }

    fn any_scheme(
        base: Url,
        http: reqwest::Client,
        timeout: Duration,
    ) -> Result<Self, FeedAddressError> {
        let endpoint =
            crate::fhir_base::under_base(base, "AuditEvent").ok_or(FeedAddressError::Base)?;
        Ok(Self {
            endpoint,
            http,
            timeout,
            credentials: None,
        })
    }

    /// This repository, each request authorized by `credentials`.
    #[must_use]
    pub fn with_credentials(mut self, credentials: Arc<dyn Credentials>) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// The `[base]/AuditEvent` URL records are posted to.
    #[must_use]
    pub fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    /// Posts `record`, one `AuditEvent` as FHIR JSON (§3.20.4.2.2).
    ///
    /// # Errors
    ///
    /// A [`FeedError`] for every answer but a `2xx`, and for no answer.
    pub async fn send(&self, record: &SecretSlice<u8>) -> Result<(), FeedError> {
        let Some(credentials) = &self.credentials else {
            return outcome(self.post(record, None).await?);
        };
        let first = credentials
            .authorization()
            .await
            .map_err(FeedError::Credentials)?;
        let status = self.post(record, Some(&first)).await?;
        if status != StatusCode::UNAUTHORIZED {
            return outcome(status);
        }
        // NOTE: RFC 6750 §3.1: a 401 for an invalid token is answered with a new token, once.
        credentials.refused(&first).await;
        let fresh = credentials
            .authorization()
            .await
            .map_err(FeedError::Credentials)?;
        outcome(self.post(record, Some(&fresh)).await?)
    }

    /// Posts `record` with the `Authorization` value `authorization`, and
    /// returns the status the repository answered.
    async fn post(
        &self,
        record: &SecretSlice<u8>,
        authorization: Option<&SecretString>,
    ) -> Result<StatusCode, FeedError> {
        let mut request = self
            .http
            .post(self.endpoint.clone())
            .header(CONTENT_TYPE, FHIR_JSON)
            .body(record.expose_secret().to_vec())
            .timeout(self.timeout);
        if let Some(value) = authorization {
            let mut header = http::HeaderValue::from_str(value.expose_secret())
                .map_err(|invalid| FeedError::Credentials(CredentialsError(Box::new(invalid))))?;
            header.set_sensitive(true);
            request = request.header(AUTHORIZATION, header);
        }
        let answer = request
            .send()
            .await
            .map_err(|error| FeedError::Transport(error.without_url()))?;
        Ok(answer.status())
    }
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
            .field("timeout", &self.timeout)
            .field("credentials", &self.credentials.is_some())
            .finish_non_exhaustive()
    }
}
