//! X.509 certificates of the internal authority and their strict profiles.
//!
//! [`parse`] admits one strict DER certificate and extracts the fields the
//! other checks need, without judging its profile. [`check_profile`] applies
//! the exact shape of one role, [`check_issued_by`] the link to an issuer and
//! [`valid_over`] the validity window. The rules mirror
//! `crates/infrastructure/src/certificates/internal_profile/profile.rs`.
//! Rationale: `docs/adr/0085-in-browser-evidence-validation.md`.

mod profile;
#[cfg(test)]
mod tests;

use der::asn1::ObjectIdentifier;
use der::oid::AssociatedOid;
use der::{Decode, Encode};
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use rsa::{Pkcs1v15Sign, RsaPublicKey};
use sha2::{Digest, Sha256};
use x509_cert::ext::pkix::{AuthorityKeyIdentifier, SubjectKeyIdentifier};
use x509_cert::name::Name;
use x509_cert::time::Time;

use crate::der_walk;
use crate::report::Finding;

pub use profile::check_profile;

/// Largest certificate admitted, in DER bytes.
pub const CERTIFICATE_LIMIT: usize = 16 * 1024;

/// `rsaEncryption`, the only admitted public key algorithm.
pub(crate) const RSA_ENCRYPTION: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");
/// `sha256WithRSAEncryption`, the only admitted certificate signature.
pub(crate) const RSA_SHA256: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.11");

/// The purpose a certificate is checked for; each has one exact shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Self-signed authority root that signs certificates and CRLs.
    Root,
    /// Server seal signer, issued with the `Partner` shape of
    /// `pki/openssl.cnf` section `v3_end_entity`.
    ServerSealSigner,
    /// Individual signing holder, with the `Declaration` shape.
    IndividualSigner,
    /// Time-stamping authority signer.
    TimeStamping,
}

/// One decoded certificate together with its exact encoding.
#[derive(Debug, Clone)]
pub struct Certificate {
    /// The complete DER certificate as supplied.
    pub der: Vec<u8>,
    /// The exact bytes of `tbsCertificate`, the input of the issuer signature.
    pub tbs_der: Vec<u8>,
    /// Content octets of the serial INTEGER, including a leading zero octet
    /// when the DER encoding needs one for the sign.
    pub serial: Vec<u8>,
    pub subject: Name,
    pub issuer: Name,
    /// Start of the validity window, Unix seconds, inclusive.
    pub not_before: i64,
    /// End of the validity window, Unix seconds, inclusive.
    pub not_after: i64,
    /// Value of the first subject key identifier extension, if any.
    pub subject_key_id: Option<Vec<u8>>,
    /// Key identifier of the first authority key identifier extension, if any.
    pub authority_key_id: Option<Vec<u8>>,
    /// The RSA key, when the key info is `rsaEncryption` with NULL parameters
    /// and decodes; any other key fails [`check_profile`].
    pub public_key: Option<RsaPublicKey>,
    /// The signature value octets.
    pub signature: Vec<u8>,
    /// The full decoded structure, for checks that need further fields.
    pub decoded: x509_cert::Certificate,
}

impl Certificate {
    /// Serial in uppercase hexadecimal without leading zeros, `0` for zero.
    pub fn serial_hex(&self) -> String {
        let hex: String = self
            .serial
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect();
        match hex.trim_start_matches('0') {
            "" => "0".to_string(),
            trimmed => trimmed.to_string(),
        }
    }
}

/// Decode one strict DER certificate.
///
/// The input must be a single complete element within the nesting bound of
/// [`der_walk::MAX_DEPTH`] that decodes as a certificate and re-encodes to
/// the same bytes; anything else is a failed `cert.malformed`. An input over
/// [`CERTIFICATE_LIMIT`] bytes is not supported. Profile rules are not
/// applied here.
pub fn parse(der: &[u8]) -> Result<Certificate, Finding> {
    if der.len() > CERTIFICATE_LIMIT {
        return Err(Finding::not_supported(
            "cert.too_large",
            format!("certificate of {} bytes", der.len()),
        ));
    }
    let malformed = |detail: &str| Finding::failed("cert.malformed", detail);
    let outer =
        der_walk::walk(der, der_walk::MAX_DEPTH).map_err(|error| malformed(error.code()))?;
    let (tbs, _) = der_walk::read(outer.value).map_err(|error| malformed(error.code()))?;
    let decoded =
        x509_cert::Certificate::from_der(der).map_err(|_| malformed("not an X.509 certificate"))?;
    if decoded.to_der().ok().as_deref() != Some(der) {
        return Err(malformed("not in canonical DER"));
    }
    let fields = &decoded.tbs_certificate;
    let extensions = fields.extensions.as_deref().unwrap_or_default();
    let first = |oid: ObjectIdentifier| {
        extensions
            .iter()
            .find(|extension| extension.extn_id == oid)
            .map(|extension| extension.extn_value.as_bytes())
    };
    let subject_key_id = match first(SubjectKeyIdentifier::OID) {
        None => None,
        Some(value) => Some(
            SubjectKeyIdentifier::from_der(value)
                .map_err(|_| malformed("subject key identifier"))?
                .0
                .as_bytes()
                .to_vec(),
        ),
    };
    let authority_key_id = match first(AuthorityKeyIdentifier::OID) {
        None => None,
        Some(value) => AuthorityKeyIdentifier::from_der(value)
            .map_err(|_| malformed("authority key identifier"))?
            .key_identifier
            .map(|id| id.as_bytes().to_vec()),
    };
    let not_before = unix(fields.validity.not_before);
    let not_after = unix(fields.validity.not_after);
    Ok(Certificate {
        der: der.to_vec(),
        tbs_der: tbs.raw.to_vec(),
        serial: fields.serial_number.as_bytes().to_vec(),
        subject: fields.subject.clone(),
        issuer: fields.issuer.clone(),
        not_before,
        not_after,
        subject_key_id,
        authority_key_id,
        public_key: rsa_key(&fields.subject_public_key_info),
        signature: decoded.signature.raw_bytes().to_vec(),
        decoded,
    })
}

/// Check that `issuer` issued `child`: the issuer name equals the issuer's
/// subject, the authority key identifier equals the issuer's subject key
/// identifier, and the issuer key verifies the RSA PKCS#1 v1.5 SHA-256
/// signature over the exact `tbsCertificate` bytes.
///
/// Neither certificate's profile is checked here; callers apply
/// [`check_profile`] to both.
pub fn check_issued_by(child: &Certificate, issuer: &Certificate) -> Result<(), Finding> {
    if child.issuer != issuer.subject {
        return Err(Finding::failed(
            "cert.issuer.name_mismatch",
            format!("issuer {} is not {}", child.issuer, issuer.subject),
        ));
    }
    if child.authority_key_id.is_none() || child.authority_key_id != issuer.subject_key_id {
        return Err(Finding::failed(
            "cert.issuer.key_id_mismatch",
            "authority key identifier differs from the issuer key identifier",
        ));
    }
    let signed = issuer
        .public_key
        .as_ref()
        .is_some_and(|key| verify_rsa_sha256(key, &child.tbs_der, &child.signature));
    if !signed {
        return Err(Finding::failed(
            "cert.issuer.signature_invalid",
            "the issuer key does not verify the certificate signature",
        ));
    }
    Ok(())
}

/// Whether the validity window contains the whole interval `from..=to`.
///
/// Both window ends are inclusive. An interval whose start is after its end
/// is never covered.
pub fn valid_over(certificate: &Certificate, from: i64, to: i64) -> bool {
    from <= to && certificate.not_before <= from && to <= certificate.not_after
}

/// SHA-256 of `bytes`, used for fingerprints and covered digests.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Verify an RSA PKCS#1 v1.5 signature with SHA-256 over `message`.
///
/// The digest is computed here. The signature must have exactly the length
/// of the key modulus: 384 bytes for the RSA-3072 keys the profiles admit.
pub fn verify_rsa_sha256(key: &RsaPublicKey, message: &[u8], signature: &[u8]) -> bool {
    signature.len() == key.size()
        && key
            .verify(
                Pkcs1v15Sign::new::<Sha256>(),
                &Sha256::digest(message),
                signature,
            )
            .is_ok()
}

fn unix(time: Time) -> i64 {
    i64::try_from(time.to_unix_duration().as_secs()).unwrap_or(i64::MAX)
}

fn rsa_key(info: &spki::SubjectPublicKeyInfoOwned) -> Option<RsaPublicKey> {
    if info.algorithm.oid != RSA_ENCRYPTION
        || !info
            .algorithm
            .parameters
            .as_ref()
            .is_some_and(|value| value.is_null())
    {
        return None;
    }
    RsaPublicKey::from_public_key_der(&info.to_der().ok()?).ok()
}
