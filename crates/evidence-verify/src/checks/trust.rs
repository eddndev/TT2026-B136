//! Certificates of a signer role, their roots and the trust of those roots.
//!
//! A root is admitted only when its SHA-256 fingerprint is pinned for the
//! purpose it serves. A root the served trust returns without being pinned
//! is indeterminate trust; any other root is not supported. A pinned root
//! missing from the served trust stays admitted and is noted. Rationale: the
//! section "Trust anchors" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use crate::certificate::{self, Certificate, Role};
use crate::policy::{Policy, Purpose};
use crate::report::{hex, Finding, TrustNote};

/// One root a signer certificate is checked against, at one instant.
pub(super) struct Anchor<'a> {
    /// The entry the root was read from.
    pub entry: &'static str,
    pub root: &'a Result<Certificate, Finding>,
    /// What the instant is, for the report.
    pub instant: &'static str,
    /// The interval the validity windows must contain, when known.
    pub at: Option<(i64, i64)>,
}

/// The signer role and the anchors it is checked against.
pub(super) struct Chain<'a> {
    pub entry: &'static str,
    pub signer: &'a Result<Certificate, Finding>,
    pub role: Role,
    pub purpose: Purpose,
    pub anchors: &'a [Anchor<'a>],
}

/// Check the strict profile of the signer and of each root, the issuance
/// link, the trust of each root and the validity windows at each instant.
/// A root that appears in several anchors with the same bytes is profiled
/// and admitted once.
pub(super) fn check_chain(chain: &Chain<'_>, policy: &Policy) -> Vec<Finding> {
    let mut findings = Vec::new();
    let signer = match chain.signer {
        Ok(signer) => {
            findings.push(profile(signer, chain.role, chain.entry));
            Some(signer)
        }
        Err(finding) => {
            findings.push(finding.clone());
            None
        }
    };
    let mut seen: Vec<&[u8]> = Vec::new();
    for anchor in chain.anchors {
        let root = match anchor.root {
            Ok(root) => root,
            Err(finding) => {
                findings.push(finding.clone());
                continue;
            }
        };
        if !seen.contains(&root.der.as_slice()) {
            seen.push(&root.der);
            findings.push(profile(root, Role::Root, anchor.entry));
            findings.push(admit_root(root, chain.purpose, policy, anchor.entry));
            if let Some(signer) = signer {
                findings.push(match certificate::check_issued_by(signer, root) {
                    Ok(()) => Finding::new(
                        crate::report::Outcome::Checked,
                        "cert.issued_by",
                        format!("{} by {}", chain.entry, anchor.entry),
                    ),
                    Err(finding) => super::in_entry(chain.entry, finding),
                });
            }
        }
        findings.push(validity(signer, root, anchor));
    }
    findings
}

fn profile(certificate: &Certificate, role: Role, entry: &str) -> Finding {
    match certificate::check_profile(certificate, role) {
        Ok(()) => Finding::new(
            crate::report::Outcome::Checked,
            "cert.profile",
            format!("{entry}: {role:?}"),
        ),
        Err(finding) => super::in_entry(entry, finding),
    }
}

/// Validity of the signer, when it decoded, and of the root over the
/// anchor's interval.
fn validity(signer: Option<&Certificate>, root: &Certificate, anchor: &Anchor<'_>) -> Finding {
    let Some((from, to)) = anchor.at else {
        return Finding::indeterminate(
            "cert.instant_unknown",
            format!("{} is not established", anchor.instant),
        );
    };
    let outside: Vec<String> = signer
        .into_iter()
        .chain(std::iter::once(root))
        .filter(|certificate| !certificate::valid_over(certificate, from, to))
        .map(|certificate| {
            format!(
                "{} valid {}..={}",
                certificate.subject, certificate.not_before, certificate.not_after
            )
        })
        .collect();
    if outside.is_empty() {
        Finding::new(
            crate::report::Outcome::Checked,
            "cert.validity",
            format!("{} {from}..={to}", anchor.instant),
        )
    } else {
        Finding::failed(
            "cert.outside_validity",
            format!(
                "at {} {from}..={to}: {}",
                anchor.instant,
                outside.join("; ")
            ),
        )
    }
}

/// Whether `root` is admitted for `purpose`.
pub(super) fn admit_root(
    root: &Certificate,
    purpose: Purpose,
    policy: &Policy,
    entry: &str,
) -> Finding {
    let fingerprint = certificate::sha256(&root.der);
    if let Some(pinned) = policy.pinned_for(&fingerprint, purpose) {
        return Finding::new(
            crate::report::Outcome::Checked,
            "trust.root_pinned",
            format!("{entry}: {} for {}", pinned.label, purpose.name()),
        );
    }
    if served_for(policy, &fingerprint, purpose) {
        return Finding::indeterminate(
            "trust.root_served_not_pinned",
            format!(
                "{entry}: {} is served for {} but not pinned",
                hex(&fingerprint),
                purpose.name()
            ),
        );
    }
    Finding::not_supported(
        "trust.root_not_pinned",
        format!(
            "{entry}: {} is not pinned for {}",
            hex(&fingerprint),
            purpose.name()
        ),
    )
}

/// Whether the served trust returns the root `fingerprint` for `purpose`.
pub(super) fn served_for(policy: &Policy, fingerprint: &[u8; 32], purpose: Purpose) -> bool {
    policy.served.as_ref().is_some_and(|served| {
        served.roots.iter().any(|root| {
            root.purposes.allows(purpose) && &certificate::sha256(&root.der) == fingerprint
        })
    })
}

/// The differences between the pinned set and the served trust, per root
/// and purpose, or a note that the route was unavailable.
pub(crate) fn route_notes(policy: &Policy) -> Vec<TrustNote> {
    let Some(served) = &policy.served else {
        return vec![TrustNote {
            code: "trust.route_unavailable",
            purpose: None,
            fingerprint: String::new(),
            detail: "the served trust was not available; only the current status changes"
                .to_owned(),
        }];
    };
    let mut notes = Vec::new();
    for pinned in &policy.pinned {
        for purpose in Purpose::ALL {
            if pinned.purposes.allows(purpose) && !served_for(policy, &pinned.sha256, purpose) {
                notes.push(TrustNote {
                    code: "trust.pinned_not_served",
                    purpose: Some(purpose),
                    fingerprint: hex(&pinned.sha256),
                    detail: format!("{} is pinned but not served", pinned.label),
                });
            }
        }
    }
    for root in &served.roots {
        let fingerprint = certificate::sha256(&root.der);
        for purpose in Purpose::ALL {
            if root.purposes.allows(purpose) && policy.pinned_for(&fingerprint, purpose).is_none() {
                notes.push(TrustNote {
                    code: "trust.served_not_pinned",
                    purpose: Some(purpose),
                    fingerprint: hex(&fingerprint),
                    detail: "served but not pinned; it is indeterminate trust".to_owned(),
                });
            }
        }
    }
    notes
}
