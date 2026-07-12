//! RSA PKCS#1 v1.5 signing and verification adapters over SHA-256 digests.
//!
//! Signatures follow the DigestInfo path of PKCS#1 v1.5: the padded message
//! embeds the SHA-256 object identifier and the 32 digest bytes computed by
//! the hashing port, so nothing is hashed twice. The choice of the `rsa`
//! crate and its accepted timing advisory are documented in
//! docs/adr/0002-rsa-signing-crate-and-advisory.md.

use domain::crypto::{
    DocumentSigner, Sha256Digest, Signature, SignatureRejection, SignatureVerification,
    SignatureVerifier,
};
use domain::DomainError;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::traits::PublicKeyParts;
use rsa::{Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey};
use sha2::Sha256;
use x509_cert::der::{DecodePem, Encode};
use x509_cert::Certificate;
use zeroize::Zeroizing;

use crate::error::CryptoError;

/// Smallest RSA modulus the signer accepts, in bits. Keys issued by the
/// pki/ scripts are 3072-bit; anything smaller is rejected outright.
pub const MIN_RSA_MODULUS_BITS: usize = 3072;

/// [`DocumentSigner`] holding an RSA private key as PEM bytes.
///
/// The key stays in its zeroizing PEM buffer between calls; it is parsed
/// into an [`RsaPrivateKey`] only while a signature is being produced and
/// dropped as soon as the call completes. The key is never logged and never
/// leaves this adapter.
pub struct RsaPkcs1Signer {
    key_pem: Zeroizing<Vec<u8>>,
}

impl RsaPkcs1Signer {
    /// Validates the PEM-encoded private key and takes ownership of it.
    ///
    /// Accepts PKCS#8 (`BEGIN PRIVATE KEY`) and PKCS#1
    /// (`BEGIN RSA PRIVATE KEY`) encodings. Fails when the PEM cannot be
    /// parsed or the modulus is shorter than [`MIN_RSA_MODULUS_BITS`].
    pub fn new(key_pem: Zeroizing<Vec<u8>>) -> Result<Self, CryptoError> {
        // The parsed key exists only inside this call; the adapter keeps
        // nothing but the zeroizing PEM buffer.
        let modulus_bits = parse_private_key_pem(&key_pem)?.size() * 8;
        if modulus_bits < MIN_RSA_MODULUS_BITS {
            return Err(CryptoError::InvalidKeyMaterial(format!(
                "rsa modulus must be at least {MIN_RSA_MODULUS_BITS} bits, got {modulus_bits}"
            )));
        }
        Ok(Self { key_pem })
    }
}

/// Debug never shows the key; it only states that one is held.
impl std::fmt::Debug for RsaPkcs1Signer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RsaPkcs1Signer(private key pem withheld)")
    }
}

impl DocumentSigner for RsaPkcs1Signer {
    fn sign(&self, digest: &Sha256Digest) -> Result<Signature, DomainError> {
        // Scope the parsed private key so it drops (and zeroizes) as soon
        // as the signature bytes exist.
        let raw = {
            let key = parse_private_key_pem(&self.key_pem).map_err(DomainError::from)?;
            key.sign(Pkcs1v15Sign::new::<Sha256>(), digest.as_bytes())
                .map_err(|err| {
                    DomainError::CryptoBackendFailure(format!("rsa signing failed: {err}"))
                })?
        };
        Signature::from_bytes(raw)
    }
}

/// [`SignatureVerifier`] for RSA PKCS#1 v1.5 over SHA-256 digests.
///
/// Key material is accepted in two PEM forms that share one path: an X509
/// certificate (the subject public key is extracted from it) or a bare
/// SubjectPublicKeyInfo public key.
#[derive(Debug, Clone, Copy, Default)]
pub struct RsaPkcs1Verifier;

impl RsaPkcs1Verifier {
    /// Creates the adapter; it holds no state.
    pub fn new() -> Self {
        Self
    }
}

impl SignatureVerifier for RsaPkcs1Verifier {
    fn verify(
        &self,
        digest: &Sha256Digest,
        signature: &Signature,
        key_material: &[u8],
    ) -> Result<SignatureVerification, DomainError> {
        let key = match parse_public_key_material(key_material) {
            Ok(key) => key,
            Err(cause) => return Ok(SignatureVerification::Invalid(cause)),
        };
        if signature.as_bytes().len() != key.size() {
            return Ok(SignatureVerification::Invalid(
                SignatureRejection::MalformedSignature(format!(
                    "signature must be {} bytes for this key, got {}",
                    key.size(),
                    signature.as_bytes().len()
                )),
            ));
        }
        // A failed PKCS#1 v1.5 check cannot tell a changed document from a
        // wrong key, so every rejection collapses into the mismatch cause.
        match key.verify(
            Pkcs1v15Sign::new::<Sha256>(),
            digest.as_bytes(),
            signature.as_bytes(),
        ) {
            Ok(()) => Ok(SignatureVerification::Valid),
            Err(_) => Ok(SignatureVerification::Invalid(
                SignatureRejection::MismatchedDocumentOrKey,
            )),
        }
    }
}

/// Reads the subject name out of a PEM certificate, in RFC 4514 form.
///
/// Offered so callers can report who a certificate belongs to without
/// parsing X509 themselves.
pub fn certificate_subject(cert_pem: &[u8]) -> Result<String, CryptoError> {
    let certificate = Certificate::from_pem(cert_pem).map_err(|err| {
        CryptoError::InvalidKeyMaterial(format!("cannot parse certificate: {err}"))
    })?;
    Ok(certificate.tbs_certificate.subject.to_string())
}

/// Extracts an RSA public key from certificate or SubjectPublicKeyInfo PEM.
fn parse_public_key_material(material: &[u8]) -> Result<RsaPublicKey, SignatureRejection> {
    let malformed = SignatureRejection::MalformedKeyMaterial;
    let text = std::str::from_utf8(material)
        .map_err(|_| malformed("verifier key material is not utf-8".to_string()))?;
    if text.contains("-----BEGIN CERTIFICATE-----") {
        let certificate = Certificate::from_pem(material)
            .map_err(|err| malformed(format!("cannot parse certificate: {err}")))?;
        let spki_der = certificate
            .tbs_certificate
            .subject_public_key_info
            .to_der()
            .map_err(|err| malformed(format!("cannot read the subject public key: {err}")))?;
        RsaPublicKey::from_public_key_der(&spki_der)
            .map_err(|err| malformed(format!("certificate does not hold an rsa key: {err}")))
    } else if text.contains("-----BEGIN PUBLIC KEY-----") {
        RsaPublicKey::from_public_key_pem(text)
            .map_err(|err| malformed(format!("cannot parse public key: {err}")))
    } else {
        Err(malformed(
            "verifier key material must hold a certificate or public key pem block".to_string(),
        ))
    }
}

/// Parses a PKCS#8 or PKCS#1 PEM private key.
fn parse_private_key_pem(pem: &[u8]) -> Result<RsaPrivateKey, CryptoError> {
    let text = std::str::from_utf8(pem)
        .map_err(|_| CryptoError::InvalidKeyMaterial("private key pem is not utf-8".to_string()))?;
    if text.contains("-----BEGIN PRIVATE KEY-----") {
        RsaPrivateKey::from_pkcs8_pem(text).map_err(|err| {
            CryptoError::InvalidKeyMaterial(format!("cannot parse pkcs#8 private key: {err}"))
        })
    } else if text.contains("-----BEGIN RSA PRIVATE KEY-----") {
        RsaPrivateKey::from_pkcs1_pem(text).map_err(|err| {
            CryptoError::InvalidKeyMaterial(format!("cannot parse pkcs#1 private key: {err}"))
        })
    } else {
        Err(CryptoError::InvalidKeyMaterial(
            "private key pem must hold a pkcs#8 or pkcs#1 block".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constructor_error(pem: &[u8]) -> String {
        RsaPkcs1Signer::new(Zeroizing::new(pem.to_vec()))
            .expect_err("the constructor must reject this input")
            .to_string()
    }

    #[test]
    fn constructor_rejects_bytes_without_a_pem_block() {
        let message = constructor_error(b"not a key at all");
        assert!(
            message.contains("pkcs#8 or pkcs#1"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn constructor_rejects_non_utf8_bytes() {
        let message = constructor_error(&[0xff, 0xfe, 0x00]);
        assert!(
            message.contains("not utf-8"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn constructor_rejects_a_corrupt_pkcs8_block() {
        let pem = "-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n";
        let message = constructor_error(pem.as_bytes());
        assert!(message.contains("pkcs#8"), "unexpected message: {message}");
    }

    fn rejection_of(material: &[u8]) -> SignatureRejection {
        let digest = Sha256Digest::from_array([0u8; 32]);
        let signature = Signature::from_bytes(vec![0u8; 384]).unwrap();
        match RsaPkcs1Verifier::new()
            .verify(&digest, &signature, material)
            .unwrap()
        {
            SignatureVerification::Invalid(cause) => cause,
            SignatureVerification::Valid => panic!("this material must never verify"),
        }
    }

    #[test]
    fn verifier_rejects_material_without_a_pem_block() {
        let cause = rejection_of(b"no pem here");
        assert!(
            matches!(&cause, SignatureRejection::MalformedKeyMaterial(m)
                if m.contains("certificate or public key")),
            "unexpected cause: {cause:?}"
        );
    }

    #[test]
    fn verifier_rejects_non_utf8_material() {
        let cause = rejection_of(&[0xff, 0xfe, 0x00]);
        assert!(
            matches!(&cause, SignatureRejection::MalformedKeyMaterial(m)
                if m.contains("not utf-8")),
            "unexpected cause: {cause:?}"
        );
    }

    #[test]
    fn verifier_rejects_a_corrupt_certificate_block() {
        let pem = "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n";
        let cause = rejection_of(pem.as_bytes());
        assert!(
            matches!(&cause, SignatureRejection::MalformedKeyMaterial(m)
                if m.contains("cannot parse certificate")),
            "unexpected cause: {cause:?}"
        );
    }

    #[test]
    fn certificate_subject_rejects_a_corrupt_certificate() {
        let pem = "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n";
        let err = certificate_subject(pem.as_bytes()).unwrap_err();
        assert!(
            err.to_string().contains("cannot parse certificate"),
            "unexpected message: {err}"
        );
    }
}
