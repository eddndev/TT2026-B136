//! Certificate revocation lists of the internal authority.
//!
//! [`parse`] admits one strict DER CRL that has the exact CRL profile;
//! [`check_issued_by`] links it to its issuer; [`covers`] and
//! [`revocation_of`] read its window and entries; [`status`] combines them
//! into the revocation outcome of one certificate over one interval. The
//! profile rules mirror `inspect` in
//! `crates/infrastructure/src/certificates/internal_profile/revocation.rs`.
//! Callers decode PEM with [`crate::pem`] first. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

mod profile;
#[cfg(test)]
mod tests;

use der::{Decode, Encode};
use x509_cert::crl::CertificateList;
use x509_cert::name::Name;
use x509_cert::time::Time;

use crate::certificate::{verify_rsa_sha256, Certificate};
use crate::der_walk;
use crate::report::{Finding, Outcome};

/// Largest CRL admitted, in DER bytes.
pub const CRL_LIMIT: usize = 1024 * 1024;

/// One CRL that passed every rule of the CRL profile not tied to an issuer.
///
/// Outside this crate values exist only through [`parse`], so the profile
/// holds; the link to an issuer is checked separately by [`check_issued_by`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Crl {
    /// The complete DER list as supplied.
    pub der: Vec<u8>,
    /// The exact bytes of `tbsCertList`, the input of the issuer signature.
    pub tbs_der: Vec<u8>,
    /// The 384 signature value octets.
    pub signature: Vec<u8>,
    /// Content octets of the CRL number, at most 8, without a sign octet.
    pub number: Vec<u8>,
    /// Start of the window, Unix seconds, inclusive.
    pub this_update: i64,
    /// End of the window, Unix seconds, inclusive; after `this_update`.
    pub next_update: i64,
    pub issuer: Name,
    /// Key identifier of the authority key identifier extension.
    pub authority_key_id: Vec<u8>,
    /// Each revoked serial, as the content octets of its INTEGER in the same
    /// form as [`Certificate::serial`], with its revocation instant in Unix
    /// seconds. Serials are unique and instants are not after `this_update`.
    pub revoked: Vec<(Vec<u8>, i64)>,
}

impl Crl {
    /// The CRL number as an unsigned integer, for ordering lists of one
    /// issuer. It fits because the profile admits at most 8 octets.
    pub fn number_value(&self) -> u64 {
        self.number
            .iter()
            .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte))
    }
}

/// Decode one strict DER CRL and apply the issuer-independent CRL profile.
///
/// The input must be a single complete element within the nesting bound of
/// [`der_walk::MAX_DEPTH`] that decodes as a CRL and re-encodes to the same
/// bytes; anything else is a failed `crl.malformed`. A profile violation is
/// a failed `crl.profile.*`. An input over [`CRL_LIMIT`] bytes is not
/// supported.
pub fn parse(der: &[u8]) -> Result<Crl, Finding> {
    if der.len() > CRL_LIMIT {
        return Err(Finding::not_supported(
            "crl.too_large",
            format!("CRL of {} bytes", der.len()),
        ));
    }
    let malformed = |detail: &str| Finding::failed("crl.malformed", detail);
    let outer =
        der_walk::walk(der, der_walk::MAX_DEPTH).map_err(|error| malformed(error.code()))?;
    let (tbs, _) = der_walk::read(outer.value).map_err(|error| malformed(error.code()))?;
    let decoded = CertificateList::from_der(der).map_err(|_| malformed("not an X.509 CRL"))?;
    if decoded.to_der().ok().as_deref() != Some(der) {
        return Err(malformed("not in canonical DER"));
    }
    let shape = profile::check(&decoded)?;
    Ok(Crl {
        der: der.to_vec(),
        tbs_der: tbs.raw.to_vec(),
        signature: decoded.signature.raw_bytes().to_vec(),
        number: shape.number,
        this_update: shape.this_update,
        next_update: shape.next_update,
        issuer: decoded.tbs_cert_list.issuer,
        authority_key_id: shape.authority_key_id,
        revoked: shape.revoked,
    })
}

/// Check that `issuer` issued `crl`: the authority key identifier equals the
/// issuer's subject key identifier, the CRL issuer name equals the issuer's
/// subject, and the issuer key verifies the RSA PKCS#1 v1.5 SHA-256
/// signature over the exact `tbsCertList` bytes. Each mismatch is a failed
/// `crl.issuer.*` finding, in that order.
///
/// The issuer's own profile is not checked here; callers apply
/// [`crate::certificate::check_profile`] with the root role.
pub fn check_issued_by(crl: &Crl, issuer: &Certificate) -> Result<(), Finding> {
    if issuer.subject_key_id.as_deref() != Some(crl.authority_key_id.as_slice()) {
        return Err(Finding::failed(
            "crl.issuer.key_id_mismatch",
            "authority key identifier differs from the issuer key identifier",
        ));
    }
    if crl.issuer != issuer.subject {
        return Err(Finding::failed(
            "crl.issuer.name_mismatch",
            format!("issuer {} is not {}", crl.issuer, issuer.subject),
        ));
    }
    let signed = issuer
        .public_key
        .as_ref()
        .is_some_and(|key| verify_rsa_sha256(key, &crl.tbs_der, &crl.signature));
    if !signed {
        return Err(Finding::failed(
            "crl.issuer.signature_invalid",
            "the issuer key does not verify the CRL signature",
        ));
    }
    Ok(())
}

/// Whether the window `thisUpdate..=nextUpdate` contains the whole interval
/// `from..=to`. An interval whose start is after its end is never covered.
pub fn covers(crl: &Crl, from: i64, to: i64) -> bool {
    from <= to && crl.this_update <= from && to <= crl.next_update
}

/// The revocation instant of `serial`, if the list names it.
///
/// `serial` is compared byte for byte in the content-octet form of
/// [`Certificate::serial`], so callers pass that field directly.
pub fn revocation_of(crl: &Crl, serial: &[u8]) -> Option<i64> {
    crl.revoked
        .iter()
        .find(|(listed, _)| listed.as_slice() == serial)
        .map(|(_, instant)| *instant)
}

/// The revocation outcome of the certificate `serial`, issued by `issuer`,
/// over the interval `from..=to`.
///
/// A CRL not issued by `issuer` or with an invalid signature is failed with
/// the [`check_issued_by`] finding. A serial the list names as revoked at or
/// before `to` is a failed `crl.revoked`, whether or not the window covers
/// the interval: the profile admits no entry extensions, so no listed
/// revocation is a hold that could be lifted, and an authentic list that
/// names it proves the revocation. Otherwise a list whose window does not
/// cover the interval is an indeterminate `crl.not_covering`, whose detail
/// names a later revocation if the list holds one. Within a covering window,
/// a serial revoked after `to` keeps the historical result as a checked
/// `crl.revoked_later` whose detail names the instant, and an unlisted
/// serial is a checked `crl.not_revoked`. Malformed and off-profile lists
/// never reach this function: [`parse`] fails them.
pub fn status(crl: &Crl, issuer: &Certificate, serial: &[u8], from: i64, to: i64) -> Finding {
    if let Err(finding) = check_issued_by(crl, issuer) {
        return finding;
    }
    let revocation = revocation_of(crl, serial);
    if let Some(instant) = revocation.filter(|instant| *instant <= to) {
        return Finding::failed(
            "crl.revoked",
            format!("revoked at {instant}, not after {to}"),
        );
    }
    if !covers(crl, from, to) {
        let listed = revocation
            .map(|instant| format!("; the list names the serial as revoked at {instant}"))
            .unwrap_or_default();
        return Finding::indeterminate(
            "crl.not_covering",
            format!(
                "window {}..={} does not cover {from}..={to}{listed}",
                crl.this_update, crl.next_update
            ),
        );
    }
    match revocation {
        Some(instant) => Finding::new(
            Outcome::Checked,
            "crl.revoked_later",
            format!("revoked at {instant}, after {to}"),
        ),
        None => Finding::checked("crl.not_revoked"),
    }
}

/// Unix seconds of an X.509 time, saturating far beyond any admitted window.
fn unix(time: Time) -> i64 {
    i64::try_from(time.to_unix_duration().as_secs()).unwrap_or(i64::MAX)
}
