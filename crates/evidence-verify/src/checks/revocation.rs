//! Revocation of each certificate role at its own instant.
//!
//! A CRL of the right issuer that does not cover the instant is
//! indeterminate; another issuer, an invalid signature, a malformed list or
//! one off the CRL profile is failed; a serial revoked at or before the
//! instant is failed; a later revocation keeps the historical result. The
//! time-stamping certificate needs a CRL from its own issuer: `crl.pem` when
//! the issuers match, otherwise its revocation is indeterminate. Rationale:
//! the section "Revocation outcomes" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use super::token::TsaChain;
use crate::certificate::Certificate;
use crate::crl::{self, Crl};
use crate::report::Finding;
use crate::timestamp::TokenFacts;

/// One signer role checked against one CRL entry.
pub(super) struct RoleRevocation<'a> {
    /// The role, for the report.
    pub role: &'static str,
    pub subject: Option<&'a Certificate>,
    pub issuer: Option<&'a Certificate>,
    pub crl: Option<&'a Crl>,
    /// The interval of the role's instant, when known.
    pub at: Option<(i64, i64)>,
}

/// The revocation outcome of one role. A CRL that did not decode is
/// reported once by the caller, so its absence here is not repeated.
pub(super) fn role_status(check: &RoleRevocation<'_>) -> Option<Finding> {
    let crl = check.crl?;
    let (Some(subject), Some(issuer)) = (check.subject, check.issuer) else {
        return Some(Finding::indeterminate(
            "revocation.certificate_unavailable",
            format!("{}: a certificate did not decode", check.role),
        ));
    };
    let Some((from, to)) = check.at else {
        return Some(Finding::indeterminate(
            "revocation.instant_unknown",
            format!("{}: the instant is not established", check.role),
        ));
    };
    Some(labelled(
        check.role,
        crl::status(crl, issuer, &subject.serial, from, to),
    ))
}

/// The revocation outcome of the time-stamping certificate at the token's
/// `genTime` interval, from `crl` when it was issued by that certificate's
/// issuer.
pub(super) fn tsa_status(
    tsa: Option<&TsaChain>,
    facts: Option<&TokenFacts>,
    crl: Option<&Crl>,
) -> Finding {
    let Some(tsa) = tsa else {
        return Finding::indeterminate(
            "revocation.tsa_unavailable",
            "the token does not chain to a pinned time-stamping root",
        );
    };
    let Some(facts) = facts else {
        return Finding::indeterminate(
            "revocation.instant_unknown",
            "time-stamping certificate: the token genTime is not established",
        );
    };
    let same_issuer = |crl: &&Crl| {
        crl.issuer == tsa.signer.issuer
            && tsa.signer.authority_key_id.as_deref() == Some(crl.authority_key_id.as_slice())
    };
    let Some(crl) = crl.filter(same_issuer) else {
        return Finding::indeterminate(
            "revocation.tsa_crl_unavailable",
            "the package has no CRL from the time-stamping certificate issuer",
        );
    };
    let (from, to) = super::token_interval(facts);
    labelled(
        "time-stamping certificate",
        crl::status(crl, &tsa.root, &tsa.signer.serial, from, to),
    )
}

fn labelled(role: &str, finding: Finding) -> Finding {
    super::in_entry(role, finding)
}
