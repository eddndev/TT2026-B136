//! Revocation of each certificate role at its own instant.
//!
//! A CRL of the right issuer that does not cover the instant is
//! indeterminate; another issuer, an invalid signature, a malformed list or
//! one off the CRL profile is failed; a serial revoked at or before the
//! instant is failed; a later revocation keeps the historical result. The
//! time-stamping certificate needs a CRL from its own issuer: `crl.pem` when
//! the issuers match, and in individual signing v2 `tsa-crl.pem`, which
//! decides for the manifest token; a revocation in either list fails.
//! Without such a list its revocation is indeterminate. Rationale:
//! the section "Revocation outcomes" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use super::token::TsaChain;
use crate::certificate::Certificate;
use crate::crl::{self, Crl};
use crate::report::{Finding, Outcome};
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

/// One CRL entry offered for the time-stamping certificate of one token.
pub(super) enum TsaList<'a> {
    /// Applied only when the list names the certificate's issuer and key
    /// identifier, as `crl.pem` is.
    IfIssuer(&'static str, &'a Crl),
    /// The list that decides for this token, as `tsa-crl.pem` does for the
    /// manifest token: always applied, so a list of another issuer fails,
    /// and its outcome is the result unless another applied list fails.
    Designated(&'static str, &'a Crl),
}

/// The revocation outcome of the time-stamping certificate of one token,
/// `role` in the report, at the token's `genTime` interval.
///
/// Each applied list is evaluated; a failure in any of them fails, so a
/// revocation in either list counts. Otherwise the designated list decides
/// when there is one, and without one a list that covers the interval
/// makes the result checked. With no applied list the revocation is
/// indeterminate. With several lists, the detail names the entry.
pub(super) fn tsa_status(
    role: &str,
    tsa: Option<&TsaChain>,
    facts: Option<&TokenFacts>,
    lists: &[TsaList<'_>],
) -> Finding {
    let Some(tsa) = tsa else {
        return Finding::indeterminate(
            "revocation.tsa_unavailable",
            format!("{role}: the token does not chain to a pinned time-stamping root"),
        );
    };
    let Some(facts) = facts else {
        return Finding::indeterminate(
            "revocation.instant_unknown",
            format!("{role}: the token genTime is not established"),
        );
    };
    let same_issuer = |crl: &Crl| {
        crl.issuer == tsa.signer.issuer
            && tsa.signer.authority_key_id.as_deref() == Some(crl.authority_key_id.as_slice())
    };
    let applied: Vec<(&str, &Crl, bool)> = lists
        .iter()
        .filter_map(|list| match list {
            TsaList::IfIssuer(entry, crl) => same_issuer(crl).then_some((*entry, *crl, false)),
            TsaList::Designated(entry, crl) => Some((*entry, *crl, true)),
        })
        .collect();
    if applied.is_empty() {
        return Finding::indeterminate(
            "revocation.tsa_crl_unavailable",
            format!("{role}: the package has no CRL from the time-stamping certificate issuer"),
        );
    }
    let (from, to) = super::token_interval(facts);
    let outcomes: Vec<(Finding, bool)> = applied
        .iter()
        .map(|(entry, crl, designated)| {
            let label = if lists.len() == 1 {
                role.to_owned()
            } else {
                format!("{role} ({entry})")
            };
            let status = crl::status(crl, &tsa.root, &tsa.signer.serial, from, to);
            (labelled(&label, status), *designated)
        })
        .collect();
    let pick = |wanted: &dyn Fn(&(Finding, bool)) -> bool| {
        outcomes
            .iter()
            .find(|outcome| wanted(outcome))
            .map(|(finding, _)| finding.clone())
    };
    pick(&|(finding, _)| finding.outcome == Outcome::Failed)
        .or_else(|| pick(&|(_, designated)| *designated))
        .or_else(|| pick(&|(finding, _)| finding.outcome == Outcome::Checked))
        .unwrap_or_else(|| outcomes[0].0.clone())
}

fn labelled(role: &str, finding: Finding) -> Finding {
    super::in_entry(role, finding)
}
