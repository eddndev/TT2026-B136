//! Local key custody and exact-byte RSA signatures for the browser.

use serde::Serialize;

mod certificate;
mod keys;
mod request;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub type Result<T> = std::result::Result<T, CryptoError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CryptoError(pub &'static str);

impl std::fmt::Display for CryptoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for CryptoError {}

#[derive(Serialize)]
pub struct GeneratedKey {
    pub encrypted_key_pem: String,
    pub csr_pem: String,
}

#[derive(Debug, Serialize)]
pub struct CertificateInfo {
    pub subject: String,
    pub certificate_sha256: String,
    pub spki_sha256: String,
    pub rsa_bits: usize,
}

pub fn generate(
    password: &str,
    cn: &str,
    organization: &str,
    country: &str,
) -> Result<GeneratedKey> {
    keys::generate(password, cn, organization, country)
}

pub fn sign(encrypted_pem: &str, password: &str, cert_der: &[u8], bytes: &[u8]) -> Result<Vec<u8>> {
    let certificate = certificate::parse(cert_der)?;
    let key = keys::unlock(encrypted_pem, password)?;
    certificate::match_key(&certificate.tbs_certificate.subject_public_key_info, &key)?;
    keys::sign(&key, bytes)
}

pub fn inspect(cert_der: &[u8]) -> Result<CertificateInfo> {
    certificate::inspect(cert_der)
}

pub fn check_key(encrypted_pem: &str, password: &str, csr_pem: &str) -> Result<bool> {
    let request = request::parse(csr_pem)?;
    let key = keys::unlock(encrypted_pem, password)?;
    certificate::match_key(&request.info.public_key, &key)?;
    Ok(true)
}
