//! The context component of individual signing: the order of the instants
//! and the consistency between registration and acceptance.
//!
//! `binding_registered_at` must not be after `accepted_at`, and the document
//! token's `genTime` not after `accepted_at` beyond the token accuracy. The
//! registration and acceptance use the same root; for equal trust revisions
//! the same CRL, otherwise an acceptance CRL with a larger number and a
//! `thisUpdate` that does not decrease, as
//! `crates/application/src/documents/individual_evidence.rs` requires. Both
//! instants are application records; in individual signing v1 nothing
//! covers them, and in v2 the manifest token covers `binding_registered_at`.
//! Rationale: the section "Instants and consistency" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

#[cfg(test)]
mod tests;

use crate::certificate::Certificate;
use crate::crl::Crl;
use crate::individual_json::IndividualSigning;
use crate::report::{Finding, Outcome};
use crate::statements::{BindingStatement, ConsentStatement};
use crate::timestamp::TokenFacts;

pub(super) struct Input<'a> {
    pub record: Option<&'a IndividualSigning>,
    pub token: Option<&'a TokenFacts>,
    pub statements: Option<&'a (BindingStatement, ConsentStatement)>,
    pub root: Option<&'a Certificate>,
    pub binding_root_der: &'a [u8],
    pub crl: Option<&'a Crl>,
    pub binding_crl: Option<&'a Crl>,
    pub binding_crl_der: &'a [u8],
    /// Whether the package has the evidence manifest, whose token covers
    /// `binding_registered_at`.
    pub manifest: bool,
}

pub(super) fn check(input: &Input<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    match input.record {
        None => findings.push(Finding::indeterminate(
            "context.records_unavailable",
            "binding_registered_at and accepted_at could not be read",
        )),
        Some(record) => {
            let detail = if input.manifest {
                "accepted_at is an application record that no time-stamp covers; the \
                 manifest token covers binding_registered_at"
            } else {
                "binding_registered_at and accepted_at are application records that no \
                 time-stamp covers"
            };
            findings.push(Finding::new(
                Outcome::Checked,
                "context.records_not_time_stamped",
                detail,
            ));
            findings.push(order(record));
            findings.push(token_order(input.token, record));
        }
    }
    findings.push(same_root(input));
    findings.push(trust_progression(input));
    findings
}

fn order(record: &IndividualSigning) -> Finding {
    if record.binding_registered_at <= record.accepted_at {
        Finding::checked("context.registration_before_acceptance")
    } else {
        Finding::failed(
            "context.registration_after_acceptance",
            format!(
                "binding_registered_at {} is after accepted_at {}",
                record.binding_registered_at, record.accepted_at
            ),
        )
    }
}

fn token_order(token: Option<&TokenFacts>, record: &IndividualSigning) -> Finding {
    let Some(facts) = token else {
        return Finding::indeterminate(
            "context.token_time_unknown",
            "the document token did not yield a verified genTime",
        );
    };
    if facts.gen_time.saturating_sub(facts.accuracy_seconds) <= record.accepted_at {
        Finding::checked("context.token_before_acceptance")
    } else {
        Finding::failed(
            "context.token_after_acceptance",
            format!(
                "genTime {} with accuracy {} s is after accepted_at {}",
                facts.gen_time, facts.accuracy_seconds, record.accepted_at
            ),
        )
    }
}

fn same_root(input: &Input<'_>) -> Finding {
    match input.root {
        None => Finding::indeterminate("context.root_unavailable", "ca.pem did not decode"),
        Some(root) if root.der == input.binding_root_der => Finding::checked("context.same_root"),
        Some(_) => Finding::failed(
            "context.root_mismatch",
            "ca.pem and binding-root.der hold different certificates",
        ),
    }
}

fn trust_progression(input: &Input<'_>) -> Finding {
    let Some((binding, consent)) = input.statements else {
        return Finding::indeterminate(
            "context.statements_unavailable",
            "the trust revisions could not be read from the statements",
        );
    };
    let Some(crl) = input.crl else {
        return Finding::indeterminate("context.trust_unavailable", "crl.pem did not decode");
    };
    if binding.trust_revision == consent.trust_revision {
        return if crl.der == input.binding_crl_der {
            Finding::checked("context.same_trust")
        } else {
            Finding::failed(
                "context.trust_mismatch",
                format!(
                    "trust revision {} has different CRLs in crl.pem and binding-crl.der",
                    consent.trust_revision
                ),
            )
        };
    }
    let Some(earlier) = input.binding_crl else {
        return Finding::indeterminate(
            "context.trust_unavailable",
            "binding-crl.der did not decode",
        );
    };
    if crl.number_value() > earlier.number_value() && crl.this_update >= earlier.this_update {
        Finding::checked("context.trust_progressed")
    } else {
        Finding::failed(
            "context.trust_order",
            format!(
                "acceptance CRL {} at {} does not follow registration CRL {} at {}",
                crl.number_value(),
                crl.this_update,
                earlier.number_value(),
                earlier.this_update
            ),
        )
    }
}
