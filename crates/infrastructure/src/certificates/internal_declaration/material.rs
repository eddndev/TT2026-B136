use der::asn1::ObjectIdentifier;
use der::oid::AssociatedOid;
use der::{Decode, Encode};
use domain::crypto::{CredentialFailure, Sha256Digest};
use rsa::{Pkcs1v15Sign, RsaPublicKey};
use sha2::{Digest, Sha256};
use x509_cert::ext::Extension;

pub(super) const CERTIFICATE_LIMIT: usize = 16 * 1024;
pub(super) const CRL_LIMIT: usize = 1024 * 1024;
pub(super) const RSA_SHA256: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.11");
pub(super) const RSA_ENCRYPTION: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");

pub(super) fn bounds(bytes: &[u8], maximum: usize) -> Result<(), CredentialFailure> {
    if bytes.len() > maximum {
        return Err(CredentialFailure::LimitExceeded);
    }
    Ok(())
}

pub(super) fn der_bytes(
    bytes: &[u8],
    maximum: usize,
    label: &str,
    malformed: CredentialFailure,
) -> Result<Vec<u8>, CredentialFailure> {
    bounds(bytes, maximum)?;
    let trimmed = bytes.trim_ascii();
    if !trimmed.starts_with(b"-----BEGIN") {
        return Ok(bytes.to_vec());
    }
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    if !trimmed.starts_with(begin.as_bytes()) || !trimmed.ends_with(end.as_bytes()) {
        return Err(malformed);
    }
    let content = &trimmed[begin.len()..trimmed.len() - end.len()];
    if content.windows(5).any(|part| part == b"-----") {
        return Err(malformed);
    }
    let (actual, der) = der::pem::decode_vec(trimmed).map_err(|_| malformed)?;
    if actual != label {
        return Err(malformed);
    }
    Ok(der)
}

pub(super) fn digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_array(Sha256::digest(bytes).into())
}

pub(super) fn signature_matches(
    key: &RsaPublicKey,
    digest: &Sha256Digest,
    signature: &[u8],
) -> bool {
    signature.len() == 384
        && key
            .verify(Pkcs1v15Sign::new::<Sha256>(), digest.as_bytes(), signature)
            .is_ok()
}

pub(super) fn extension<'a, T: Decode<'a> + AssociatedOid>(
    extensions: &'a [Extension],
    critical: bool,
    unsupported: CredentialFailure,
) -> Result<T, CredentialFailure> {
    let extension = extensions
        .iter()
        .find(|extension| extension.extn_id == T::OID)
        .ok_or(unsupported)?;
    if extension.critical != critical {
        return Err(unsupported);
    }
    T::from_der(extension.extn_value.as_bytes()).map_err(|_| unsupported)
}

pub(super) fn inventory(
    extensions: &[Extension],
    allowed: &[ObjectIdentifier],
    unsupported: CredentialFailure,
) -> Result<(), CredentialFailure> {
    if extensions.len() != allowed.len()
        || allowed
            .iter()
            .any(|oid| extensions.iter().filter(|ext| ext.extn_id == *oid).count() != 1)
    {
        return Err(unsupported);
    }
    Ok(())
}

pub(super) fn encoded<T: Encode>(
    value: &T,
    malformed: CredentialFailure,
) -> Result<Vec<u8>, CredentialFailure> {
    value.to_der().map_err(|_| malformed)
}
