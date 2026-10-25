// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The bearer token of the OAuth 2.0 client credentials grant as the
//! [`Credentials`] source of a FHIR Feed repository (feature `smart`).
//!
//! `ferrohealth-smart`'s `TokenSource` fetches the token, caches it and
//! renews it before it expires. [`Bearer`] hands its `Authorization` value to
//! each Feed request and, when the repository answers `401`, drops the
//! refused token so the request is sent once more under a fresh one, as RFC
//! 6750 §3.1 has a client do on `invalid_token`.

use std::sync::Arc;

use ferrohealth_smart::client::TokenError;
use ferrohealth_smart::client::cache::TokenSource;
use http::HeaderValue;
use secrecy::{ExposeSecret, SecretString};

use super::feed::{Credentials, CredentialsError};

/// A client credentials token source as a Feed [`Credentials`] source.
#[derive(Debug, Clone)]
pub struct Bearer(TokenSource);

impl From<TokenSource> for Bearer {
    fn from(source: TokenSource) -> Self {
        Self(source)
    }
}

#[async_trait::async_trait]
impl Credentials for Bearer {
    async fn authorization(&self) -> Result<SecretString, CredentialsError> {
        let token = self
            .0
            .token()
            .await
            .map_err(|error| CredentialsError(Box::new(SharedError(error))))?;
        let value = token
            .header()
            .to_str()
            .map_err(|error| CredentialsError(Box::new(error)))?;
        Ok(SecretString::from(value.to_owned()))
    }

    async fn refused(&self, refused: &SecretString) {
        // NOTE: every value this source gives is a header value, so one that does not parse
        // is none it holds, and there is nothing to drop.
        if let Ok(stale) = HeaderValue::from_str(refused.expose_secret()) {
            self.0.invalidate(&stale);
        }
    }
}

/// The token error the callers that waited on one fetch share.
#[derive(Debug)]
struct SharedError(Arc<TokenError>);

impl std::fmt::Display for SharedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for SharedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}
