//! The exact CRL shape that does not depend on the issuer.
//!
//! The rules and their order follow `inspect` in
//! `crates/infrastructure/src/certificates/internal_profile/revocation.rs`,
//! without its issuer link and validity instant, which the parent module applies
//! separately. A violation is a failed `crl.profile.*` finding.
//! Rationale: `docs/adr/0085-in-browser-evidence-validation.md`.

use std::collections::BTreeSet;

use der::asn1::ObjectIdentifier;
use der::oid::AssociatedOid;
use der::Decode;
use x509_cert::crl::CertificateList;
use x509_cert::ext::pkix::{AuthorityKeyIdentifier, CrlNumber};
use x509_cert::ext::Extension;
use x509_cert::Version;

use super::unix;
use crate::certificate::RSA_SHA256;
use crate::report::Finding;

/// Length of an RSA-3072 signature, the only admitted CRL signature size.
const SIGNATURE_LENGTH: usize = 384;
/// Most content octets admitted in the CRL number.
const MAX_NUMBER_LENGTH: usize = 8;
/// Most revoked entries admitted in one list.
pub(super) const MAX_ENTRIES: usize = 10_000;
/// Most content octets admitted in a revoked serial number.
const MAX_SERIAL_LENGTH: usize = 20;

/// The fields of a list that passed every issuer-independent rule.
pub(super) struct Shape {
    pub number: Vec<u8>,
    pub authority_key_id: Vec<u8>,
    pub this_update: i64,
    pub next_update: i64,
    pub revoked: Vec<(Vec<u8>, i64)>,
}

fn profile(code: &'static str, detail: impl Into<String>) -> Finding {
    Finding::failed(code, detail)
}

/// Check the issuer-independent CRL profile and extract its fields.
///
/// The list must be v2, signed with `sha256WithRSAEncryption` with NULL
/// parameters in both algorithm fields and a 384-byte signature without
/// unused bits, carry exactly a non-critical authority key identifier with a
/// key identifier only and a non-critical CRL number of at most 8 octets,
/// have a `nextUpdate` after `thisUpdate`, and hold at most 10,000 entries
/// with unique non-zero serials of at most 20 octets, no entry extensions and
/// revocation dates not after `thisUpdate`.
pub(super) fn check(list: &CertificateList) -> Result<Shape, Finding> {
    let tbs = &list.tbs_cert_list;
    if tbs.version != Version::V2 {
        return Err(profile("crl.profile.version", "not a version 2 CRL"));
    }
    let null_parameters = tbs
        .signature
        .parameters
        .as_ref()
        .is_some_and(|value| value.is_null());
    if tbs.signature != list.signature_algorithm
        || tbs.signature.oid != RSA_SHA256
        || !null_parameters
    {
        return Err(profile(
            "crl.profile.algorithm",
            "not sha256WithRSAEncryption with NULL parameters in both fields",
        ));
    }
    if list.signature.as_bytes().is_none() || list.signature.raw_bytes().len() != SIGNATURE_LENGTH {
        return Err(profile(
            "crl.profile.signature_length",
            "the signature is not 384 whole octets",
        ));
    }
    let extensions = tbs.crl_extensions.as_deref().unwrap_or_default();
    check_inventory(extensions)?;
    let authority_key_id = key_identifier(extensions)?;
    let number = number(extensions)?;
    let this_update = unix(tbs.this_update);
    let next_update = match tbs.next_update {
        Some(time) if unix(time) > this_update => unix(time),
        _ => {
            return Err(profile(
                "crl.profile.next_update",
                "nextUpdate is absent or not after thisUpdate",
            ))
        }
    };
    let revoked = entries(list, this_update)?;
    Ok(Shape {
        number,
        authority_key_id,
        this_update,
        next_update,
        revoked,
    })
}

fn check_inventory(extensions: &[Extension]) -> Result<(), Finding> {
    let allowed: [ObjectIdentifier; 2] = [AuthorityKeyIdentifier::OID, CrlNumber::OID];
    let exact = extensions.len() == allowed.len()
        && allowed.iter().all(|oid| {
            extensions
                .iter()
                .filter(|extension| extension.extn_id == *oid)
                .count()
                == 1
        });
    if !exact {
        return Err(profile(
            "crl.profile.extensions",
            "extensions are not exactly the authority key identifier and the CRL number",
        ));
    }
    Ok(())
}

/// Decode the single non-critical extension of type `T`.
fn non_critical<'a, T: Decode<'a> + AssociatedOid>(extensions: &'a [Extension]) -> Option<T> {
    let extension = extensions
        .iter()
        .find(|extension| extension.extn_id == T::OID)?;
    if extension.critical {
        return None;
    }
    T::from_der(extension.extn_value.as_bytes()).ok()
}

fn key_identifier(extensions: &[Extension]) -> Result<Vec<u8>, Finding> {
    let invalid = || {
        profile(
            "crl.profile.authority_key_id",
            "the authority key identifier is critical, malformed or not key-identifier-only",
        )
    };
    let value: AuthorityKeyIdentifier = non_critical(extensions).ok_or_else(invalid)?;
    if value.authority_cert_issuer.is_some() || value.authority_cert_serial_number.is_some() {
        return Err(invalid());
    }
    let key_id = value.key_identifier.ok_or_else(invalid)?;
    Ok(key_id.as_bytes().to_vec())
}

fn number(extensions: &[Extension]) -> Result<Vec<u8>, Finding> {
    let value: Option<CrlNumber> = non_critical(extensions);
    match value {
        Some(number) if number.0.as_bytes().len() <= MAX_NUMBER_LENGTH => {
            Ok(number.0.as_bytes().to_vec())
        }
        _ => Err(profile(
            "crl.profile.number",
            "the CRL number is critical, malformed or longer than 8 octets",
        )),
    }
}

fn entries(list: &CertificateList, this_update: i64) -> Result<Vec<(Vec<u8>, i64)>, Finding> {
    let entries = list
        .tbs_cert_list
        .revoked_certificates
        .as_deref()
        .unwrap_or_default();
    if entries.len() > MAX_ENTRIES {
        return Err(profile(
            "crl.profile.entry_limit",
            format!("{} entries, at most {MAX_ENTRIES}", entries.len()),
        ));
    }
    let mut revoked: Vec<(Vec<u8>, i64)> = Vec::with_capacity(entries.len());
    let mut seen: BTreeSet<&[u8]> = BTreeSet::new();
    for entry in entries {
        let serial = entry.serial_number.as_bytes();
        if serial.len() > MAX_SERIAL_LENGTH
            || serial.iter().all(|byte| *byte == 0)
            || !seen.insert(serial)
        {
            return Err(profile(
                "crl.profile.entry_serial",
                "a revoked serial is zero, longer than 20 octets or repeated",
            ));
        }
        if entry.crl_entry_extensions.is_some() {
            return Err(profile(
                "crl.profile.entry_extensions",
                "a revoked entry carries extensions",
            ));
        }
        let revocation = unix(entry.revocation_date);
        if revocation > this_update {
            return Err(profile(
                "crl.profile.revocation_date",
                format!("revocation {revocation} after thisUpdate {this_update}"),
            ));
        }
        revoked.push((serial.to_vec(), revocation));
    }
    Ok(revoked)
}
