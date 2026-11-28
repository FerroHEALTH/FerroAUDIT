// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The TLS of the syslog hop: a CA and a `localhost` leaf minted at start,
//! the listener presenting the leaf and the sender trusting the CA.

use std::error::Error;
use std::sync::Arc;

use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ClientConfig, RootCertStore, ServerConfig};

/// The minted CA and leaf.
pub(crate) struct Pki {
    ca: CertificateDer<'static>,
    leaf: CertificateDer<'static>,
    key: Vec<u8>,
}

impl std::fmt::Debug for Pki {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pki").finish_non_exhaustive()
    }
}

impl Pki {
    /// A P-256 CA and the `localhost` leaf it signs.
    ///
    /// # Errors
    ///
    /// The `rcgen` error of a key or certificate that cannot be made.
    pub(crate) fn mint() -> Result<Self, rcgen::Error> {
        let algorithm = &rcgen::PKCS_ECDSA_P256_SHA256;
        let mut ca = CertificateParams::new(Vec::<String>::new())?;
        ca.distinguished_name
            .push(DnType::CommonName, "perf-arr CA");
        ca.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let ca_key = KeyPair::generate_for(algorithm)?;
        let ca_cert = ca.self_signed(&ca_key)?;

        let mut leaf = CertificateParams::new(vec!["localhost".to_owned()])?;
        leaf.distinguished_name
            .push(DnType::CommonName, "localhost");
        leaf.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        leaf.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let key = KeyPair::generate_for(algorithm)?;
        let leaf = leaf.signed_by(&key, &Issuer::new(ca, ca_key))?;
        Ok(Self {
            ca: ca_cert.der().clone(),
            leaf: leaf.der().clone(),
            key: key.serialize_der(),
        })
    }

    /// The listener's configuration, presenting the leaf.
    ///
    /// # Errors
    ///
    /// The `rustls` error of a configuration the provider refuses.
    pub(crate) fn server(&self) -> Result<Arc<ServerConfig>, rustls::Error> {
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let config = ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(
                vec![self.leaf.clone()],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.key.clone())),
            )?;
        Ok(Arc::new(config))
    }

    /// The sender's configuration, trusting the CA alone.
    ///
    /// # Errors
    ///
    /// The `rustls` error of a CA or configuration the provider refuses.
    pub(crate) fn client(&self) -> Result<Arc<ClientConfig>, Box<dyn Error>> {
        let mut roots = RootCertStore::empty();
        roots.add(self.ca.clone())?;
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(Arc::new(config))
    }
}
