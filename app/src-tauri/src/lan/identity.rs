//! This device's TLS identity for LAN sync: a self-signed certificate whose
//! fingerprint travels in the pairing code, so peers can pin it (D34).

use std::path::Path;
use std::sync::Arc;

use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

const CERT_FILE: &str = "sync-cert.der";
const KEY_FILE: &str = "sync-key.der";

pub struct Identity {
    cert: Vec<u8>,
    key: Vec<u8>,
    fingerprint: String,
}

impl Identity {
    /// Reuses the stored certificate, so paired devices keep trusting this one.
    pub fn load_or_create(dir: &Path) -> Result<Self> {
        let (cert_path, key_path) = (dir.join(CERT_FILE), dir.join(KEY_FILE));
        if let (Ok(cert), Ok(key)) = (std::fs::read(&cert_path), std::fs::read(&key_path)) {
            return Ok(Self::from_der(cert, key));
        }
        let identity = Self::generate()?;
        std::fs::create_dir_all(dir)?;
        std::fs::write(&cert_path, &identity.cert)?;
        write_private(&key_path, &identity.key)?;
        Ok(identity)
    }

    pub fn generate() -> Result<Self> {
        let key = rcgen::KeyPair::generate().map_err(|_| Error::Tls("key generation"))?;
        let params = rcgen::CertificateParams::new(vec!["kosmotheka.local".to_owned()])
            .map_err(|_| Error::Tls("certificate parameters"))?;
        let cert = params
            .self_signed(&key)
            .map_err(|_| Error::Tls("self-signing"))?;
        Ok(Self::from_der(cert.der().to_vec(), key.serialize_der()))
    }

    fn from_der(cert: Vec<u8>, key: Vec<u8>) -> Self {
        let fingerprint = fingerprint(&cert);
        Self {
            cert,
            key,
            fingerprint,
        }
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn server_config(&self) -> Result<Arc<ServerConfig>> {
        let config =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|_| Error::Tls("protocol versions"))?
                .with_no_client_auth()
                .with_single_cert(
                    vec![CertificateDer::from(self.cert.clone())],
                    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.key.clone())),
                )
                .map_err(|_| Error::Tls("server certificate"))?;
        Ok(Arc::new(config))
    }
}

/// sha256 of the certificate, in hex: what peers pin.
pub fn fingerprint(der: &[u8]) -> String {
    Sha256::digest(der)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    Ok(std::fs::write(path, bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_created_once_and_reloaded() {
        let dir = std::env::temp_dir().join(format!("library-identity-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let first = Identity::load_or_create(&dir).unwrap();
        let second = Identity::load_or_create(&dir).unwrap();

        assert_eq!(first.fingerprint(), second.fingerprint());
        assert_eq!(first.fingerprint().len(), 64);
        assert!(first.server_config().is_ok());
    }
}
