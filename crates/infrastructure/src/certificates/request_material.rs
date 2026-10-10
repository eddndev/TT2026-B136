//! Strict bounded PKCS#10 public request admission before CA mutation.

use der::asn1::ObjectIdentifier;
use der::{Decode, Encode};
use domain::DomainError;
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use rsa::{Pkcs1v15Sign, RsaPublicKey};
use sha2::{Digest, Sha256};
use x509_cert::request::CertReq;

pub(super) const REQUEST_LIMIT: usize = 16 * 1024;
const RSA_SHA256: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.11");
const RSA_ENCRYPTION: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");

/// Returns the exact DER SubjectPublicKeyInfo after verifying possession.
pub(super) fn inspect(input: &[u8]) -> Result<Vec<u8>, DomainError> {
    if input.is_empty() || input.len() > REQUEST_LIMIT {
        return Err(failure(
            "must contain at most 16 KiB of public PEM material",
        ));
    }
    let pem = input.trim_ascii();
    let begin = b"-----BEGIN CERTIFICATE REQUEST-----";
    let end = b"-----END CERTIFICATE REQUEST-----";
    if !pem.starts_with(begin) || !pem.ends_with(end) {
        return Err(failure("must be one public PKCS#10 PEM block"));
    }
    let content = &pem[begin.len()..pem.len() - end.len()];
    if content.windows(5).any(|part| part == b"-----") {
        return Err(failure("must be one public PKCS#10 PEM block"));
    }
    let (label, der) = der::pem::decode_vec(pem).map_err(|_| failure("PEM is malformed"))?;
    if label != "CERTIFICATE REQUEST" {
        return Err(failure("PEM label is unsupported"));
    }
    let request = CertReq::from_der(&der).map_err(|_| failure("DER is malformed"))?;
    if request.to_der().map_err(|_| failure("cannot be encoded"))? != der {
        return Err(failure("DER is not canonical"));
    }
    let algorithm = &request.algorithm;
    let spki = &request.info.public_key;
    if request.info.subject.is_empty()
        || algorithm.oid != RSA_SHA256
        || !algorithm
            .parameters
            .as_ref()
            .is_some_and(|value| value.is_null())
        || spki.algorithm.oid != RSA_ENCRYPTION
        || !spki
            .algorithm
            .parameters
            .as_ref()
            .is_some_and(|value| value.is_null())
        || request.signature.as_bytes().is_none()
        || request.signature.raw_bytes().len() != 384
    {
        return Err(failure(
            "requires RSA-3072 with SHA-256 and an identified subject",
        ));
    }
    let encoded = spki
        .to_der()
        .map_err(|_| failure("public key is malformed"))?;
    let key = RsaPublicKey::from_public_key_der(&encoded)
        .map_err(|_| failure("public key is not RSA"))?;
    if key.n().bits() != 3072 || key.e().to_bytes_be() != [1, 0, 1] {
        return Err(failure("requires RSA-3072 with public exponent 65537"));
    }
    let signed = request
        .info
        .to_der()
        .map_err(|_| failure("proof is malformed"))?;
    key.verify(
        Pkcs1v15Sign::new::<Sha256>(),
        &Sha256::digest(signed),
        request.signature.raw_bytes(),
    )
    .map_err(|_| failure("proof of possession is invalid"))?;
    Ok(encoded)
}

pub(super) fn validate_name(name: &str) -> Result<(), DomainError> {
    if name.is_empty()
        || name.len() > 64
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err(failure(
            "output name must be a safe ASCII identifier of at most 64 bytes",
        ));
    }
    Ok(())
}

pub(super) fn failure(message: &str) -> DomainError {
    DomainError::CertificateAuthorityFailure(format!("certificate request {message}"))
}
