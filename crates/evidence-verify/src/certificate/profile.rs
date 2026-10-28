//! Exact certificate shapes per role.
//!
//! The rules and their order follow
//! `crates/infrastructure/src/certificates/internal_profile/profile.rs`, so a
//! certificate with several defects reports the first one that file checks.
//! Rationale: `docs/adr/0085-in-browser-evidence-validation.md`.

use der::asn1::ObjectIdentifier;
use der::oid::AssociatedOid;
use der::{Decode, Encode};
use rsa::traits::PublicKeyParts;
use sha1::{Digest, Sha1};
use x509_cert::ext::pkix::{
    AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage, KeyUsages,
    SubjectKeyIdentifier,
};
use x509_cert::ext::Extension;
use x509_cert::Version;

use super::{verify_rsa_sha256, Certificate, Role, RSA_ENCRYPTION, RSA_SHA256};
use crate::report::Finding;

const CLIENT_AUTH: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.2");
const EMAIL_PROTECTION: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.4");
const TIME_STAMPING: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.8");

const ROOT_EXTENSIONS: [ObjectIdentifier; 3] = [
    BasicConstraints::OID,
    KeyUsage::OID,
    SubjectKeyIdentifier::OID,
];
const DECLARATION_EXTENSIONS: [ObjectIdentifier; 4] = [
    BasicConstraints::OID,
    KeyUsage::OID,
    SubjectKeyIdentifier::OID,
    AuthorityKeyIdentifier::OID,
];
const EKU_EXTENSIONS: [ObjectIdentifier; 5] = [
    BasicConstraints::OID,
    KeyUsage::OID,
    SubjectKeyIdentifier::OID,
    AuthorityKeyIdentifier::OID,
    ExtendedKeyUsage::OID,
];

/// Check that `certificate` has exactly the shape of `role`.
///
/// Every role requires X.509 v3 without unique identifiers, non-empty names,
/// `sha256WithRSAEncryption` with NULL parameters in both algorithm fields,
/// a 384-byte signature, a non-zero serial of at most 20 content octets, an
/// RSA-3072 key with exponent 65537 and NULL parameters, the exact extension
/// inventory of the role with its criticality, the subject key identifier
/// equal to the SHA-1 of the key bits and `notAfter` not before `notBefore`.
/// Non-root roles require a key-identifier-only authority key identifier of
/// 20 bytes; the root must be self-signed. A violation is a failed
/// `cert.profile.*` finding.
pub fn check_profile(certificate: &Certificate, role: Role) -> Result<(), Finding> {
    check_header(certificate)?;
    check_key(certificate)?;
    let extensions = certificate
        .decoded
        .tbs_certificate
        .extensions
        .as_deref()
        .ok_or_else(|| violation("cert.profile.extensions", "no extensions"))?;
    let allowed: &[ObjectIdentifier] = match role {
        Role::Root => &ROOT_EXTENSIONS,
        Role::IndividualSigner => &DECLARATION_EXTENSIONS,
        Role::ServerSealSigner | Role::TimeStamping => &EKU_EXTENSIONS,
    };
    if extensions.len() != allowed.len()
        || allowed
            .iter()
            .any(|oid| extensions.iter().filter(|ext| ext.extn_id == *oid).count() != 1)
    {
        return Err(violation(
            "cert.profile.extensions",
            "extension set differs from the role",
        ));
    }
    check_usages(extensions, role)?;
    check_key_identifiers(certificate, extensions, role)?;
    if certificate.not_after < certificate.not_before {
        return Err(violation(
            "cert.profile.validity",
            "notAfter precedes notBefore",
        ));
    }
    if role == Role::Root {
        let signed = certificate.public_key.as_ref().is_some_and(|key| {
            verify_rsa_sha256(key, &certificate.tbs_der, &certificate.signature)
        });
        if certificate.issuer != certificate.subject || !signed {
            return Err(violation(
                "cert.profile.self_signature",
                "root is not self-signed",
            ));
        }
    }
    Ok(())
}

fn violation(code: &'static str, detail: &str) -> Finding {
    Finding::failed(code, detail)
}

fn check_header(certificate: &Certificate) -> Result<(), Finding> {
    let decoded = &certificate.decoded;
    let tbs = &decoded.tbs_certificate;
    if tbs.version != Version::V3 {
        return Err(violation(
            "cert.profile.version",
            "not an X.509 v3 certificate",
        ));
    }
    if tbs.issuer_unique_id.is_some() || tbs.subject_unique_id.is_some() {
        return Err(violation(
            "cert.profile.unique_ids",
            "unique identifiers are present",
        ));
    }
    if tbs.subject.is_empty() || tbs.issuer.is_empty() {
        return Err(violation(
            "cert.profile.empty_name",
            "subject or issuer is empty",
        ));
    }
    if tbs.signature != decoded.signature_algorithm
        || tbs.signature.oid != RSA_SHA256
        || !tbs
            .signature
            .parameters
            .as_ref()
            .is_some_and(|value| value.is_null())
    {
        return Err(violation(
            "cert.profile.signature_algorithm",
            "signature is not sha256WithRSAEncryption with NULL parameters",
        ));
    }
    if decoded.signature.as_bytes().is_none() || certificate.signature.len() != 384 {
        return Err(violation(
            "cert.profile.signature_length",
            "signature is not 384 bytes",
        ));
    }
    if certificate.serial.len() > 20 || certificate.serial.iter().all(|byte| *byte == 0) {
        return Err(violation(
            "cert.profile.serial",
            "serial is zero or longer than 20 bytes",
        ));
    }
    Ok(())
}

fn check_key(certificate: &Certificate) -> Result<(), Finding> {
    let algorithm = &certificate
        .decoded
        .tbs_certificate
        .subject_public_key_info
        .algorithm;
    if algorithm.oid != RSA_ENCRYPTION
        || !algorithm
            .parameters
            .as_ref()
            .is_some_and(|value| value.is_null())
    {
        return Err(violation(
            "cert.profile.key_algorithm",
            "key is not rsaEncryption with NULL parameters",
        ));
    }
    let key = certificate
        .public_key
        .as_ref()
        .ok_or_else(|| violation("cert.profile.key_algorithm", "RSA key does not decode"))?;
    if key.n().bits() != 3072 {
        return Err(violation("cert.profile.key_size", "key is not RSA-3072"));
    }
    if key.e().to_bytes_be() != [1, 0, 1] {
        return Err(violation(
            "cert.profile.key_exponent",
            "exponent is not 65537",
        ));
    }
    Ok(())
}

fn check_usages(extensions: &[Extension], role: Role) -> Result<(), Finding> {
    let eku = |critical| {
        extension::<ExtendedKeyUsage>(extensions, critical)
            .ok_or_else(|| violation("cert.profile.extended_key_usage", "extended key usage form"))
    };
    let eku_mismatch = || {
        violation(
            "cert.profile.extended_key_usage",
            "extended key usage differs",
        )
    };
    match role {
        Role::ServerSealSigner => {
            let usage = eku(false)?;
            if usage.0.len() != 2
                || !usage.0.contains(&CLIENT_AUTH)
                || !usage.0.contains(&EMAIL_PROTECTION)
            {
                return Err(eku_mismatch());
            }
        }
        Role::TimeStamping => {
            if eku(true)?.0.as_slice() != [TIME_STAMPING] {
                return Err(eku_mismatch());
            }
        }
        Role::Root | Role::IndividualSigner => {}
    }
    let constraints = extension::<BasicConstraints>(extensions, true)
        .ok_or_else(|| violation("cert.profile.basic_constraints", "basic constraints form"))?;
    if constraints.ca != (role == Role::Root) || constraints.path_len_constraint.is_some() {
        return Err(violation(
            "cert.profile.basic_constraints",
            "basic constraints differ",
        ));
    }
    let usage = extension::<KeyUsage>(extensions, true)
        .ok_or_else(|| violation("cert.profile.key_usage", "key usage form"))?;
    let expected = match role {
        Role::Root => KeyUsages::KeyCertSign | KeyUsages::CRLSign,
        Role::TimeStamping => KeyUsages::DigitalSignature.into(),
        Role::ServerSealSigner | Role::IndividualSigner => {
            KeyUsages::DigitalSignature | KeyUsages::NonRepudiation
        }
    };
    if usage.0 != expected {
        return Err(violation("cert.profile.key_usage", "key usage differs"));
    }
    Ok(())
}

fn check_key_identifiers(
    certificate: &Certificate,
    extensions: &[Extension],
    role: Role,
) -> Result<(), Finding> {
    let mismatch = || {
        violation(
            "cert.profile.subject_key_id",
            "not the SHA-1 of the public key",
        )
    };
    let ski = extension::<SubjectKeyIdentifier>(extensions, false).ok_or_else(mismatch)?;
    let key_bits = certificate
        .decoded
        .tbs_certificate
        .subject_public_key_info
        .subject_public_key
        .as_bytes()
        .ok_or_else(mismatch)?;
    // OpenSSL derives this identifier with SHA-1 over the key bits; the
    // signatures and fingerprints that bind material use SHA-256.
    if ski.0.as_bytes() != Sha1::digest(key_bits).as_slice() {
        return Err(mismatch());
    }
    if role == Role::Root {
        return Ok(());
    }
    let malformed = || {
        violation(
            "cert.profile.authority_key_id",
            "not a bare 20-byte key identifier",
        )
    };
    let aki = extension::<AuthorityKeyIdentifier>(extensions, false).ok_or_else(malformed)?;
    if aki.authority_cert_issuer.is_some() || aki.authority_cert_serial_number.is_some() {
        return Err(malformed());
    }
    match aki.key_identifier {
        Some(value) if value.as_bytes().len() == 20 => Ok(()),
        _ => Err(malformed()),
    }
}

/// Decode the first extension of type `T` when its criticality is `critical`
/// and its value is in DER.
///
/// The outer round trip of [`super::parse`] does not reach inside the OCTET
/// STRING that holds an extension value, and the decoders accept forms DER
/// forbids, such as an encoded DEFAULT `cA FALSE` or key usage bits padded
/// or extended past the named bits, so the value must re-encode to the same
/// bytes.
fn extension<'a, T: Decode<'a> + Encode + AssociatedOid>(
    extensions: &'a [Extension],
    critical: bool,
) -> Option<T> {
    let extension = extensions
        .iter()
        .find(|extension| extension.extn_id == T::OID)?;
    if extension.critical != critical {
        return None;
    }
    let raw = extension.extn_value.as_bytes();
    let value = T::from_der(raw).ok()?;
    (value.to_der().ok()?.as_slice() == raw).then_some(value)
}
