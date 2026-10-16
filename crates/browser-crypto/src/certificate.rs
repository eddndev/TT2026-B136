use crate::{CertificateInfo, CryptoError, Result};
use der::{Decode, Encode};
use pkcs8::spki::SubjectPublicKeyInfoOwned;
use rsa::{
    pkcs1::{DecodeRsaPublicKey, EncodeRsaPublicKey},
    traits::PublicKeyParts,
};
use sha2::{Digest, Sha256};
use x509_cert::Certificate;

pub(crate) fn parse(cert_der: &[u8]) -> Result<Certificate> {
    if cert_der.len() > 16_384 {
        return Err(CryptoError("input_too_large"));
    }
    let certificate =
        Certificate::from_der(cert_der).map_err(|_| CryptoError("invalid_certificate"))?;
    public_key(&certificate.tbs_certificate.subject_public_key_info)?;
    Ok(certificate)
}

pub(crate) fn public_key(spki: &SubjectPublicKeyInfoOwned) -> Result<rsa::RsaPublicKey> {
    if spki.algorithm.oid != rsa::pkcs1::ALGORITHM_OID
        || spki
            .algorithm
            .parameters
            .as_ref()
            .is_some_and(|parameters| !parameters.is_null())
    {
        return Err(CryptoError("unsupported_key"));
    }
    let encoded = spki
        .subject_public_key
        .as_bytes()
        .ok_or(CryptoError("unsupported_key"))?;
    let public =
        rsa::RsaPublicKey::from_pkcs1_der(encoded).map_err(|_| CryptoError("unsupported_key"))?;
    if public.n().bits() != 3072 {
        return Err(CryptoError("unsupported_key"));
    }
    Ok(public)
}

pub(crate) fn match_key(spki: &SubjectPublicKeyInfoOwned, key: &ring::rsa::KeyPair) -> Result<()> {
    let public = public_key(spki)?
        .to_pkcs1_der()
        .map_err(|_| CryptoError("unsupported_key"))?;
    if public.as_bytes() != key.public().as_ref() {
        return Err(CryptoError("key_mismatch"));
    }
    Ok(())
}

pub(crate) fn inspect(cert_der: &[u8]) -> Result<CertificateInfo> {
    let certificate = parse(cert_der)?;
    let spki = certificate
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|_| CryptoError("invalid_certificate"))?;
    Ok(CertificateInfo {
        subject: certificate.tbs_certificate.subject.to_string(),
        certificate_sha256: format!("{:x}", Sha256::digest(cert_der)),
        spki_sha256: format!("{:x}", Sha256::digest(spki)),
        rsa_bits: 3072,
    })
}
