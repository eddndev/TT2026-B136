//! The informational current status: validity windows and revocation at the
//! caller's instant, which comes from the client clock.
//!
//! Revocation uses a served CRL of the certificate's issuer when the trust
//! route supplied one, the package CRL of that issuer otherwise. Nothing
//! here changes the historical result. Rationale: the section "Component
//! results and the overall result" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use super::token::TsaChain;
use crate::certificate::{self, Certificate};
use crate::crl::{self, Crl};
use crate::policy::Policy;
use crate::report::{Finding, Outcome};

/// One certificate of the package and, for non-root roles, its issuer.
pub(super) struct Subject<'a> {
    pub role: &'static str,
    pub certificate: &'a Certificate,
    pub issuer: Option<&'a Certificate>,
}

/// The current status of `subjects` at `policy.current_instant`, with
/// `package_crls` as the fallback lists.
pub(super) fn status(
    subjects: &[Subject<'_>],
    package_crls: &[&Crl],
    policy: &Policy,
) -> Vec<Finding> {
    let now = policy.current_instant;
    let mut findings = vec![Finding::new(
        Outcome::Checked,
        "current.instant",
        format!("{now}, read from the client clock"),
    )];
    let mut served: Vec<Crl> = Vec::new();
    match &policy.served {
        None => findings.push(Finding::indeterminate(
            "current.trust_route_unavailable",
            "the served trust was not available; package CRLs are used",
        )),
        Some(trust) => {
            for der in &trust.crls {
                match crl::parse(der) {
                    Ok(list) => served.push(list),
                    Err(finding) => findings.push(Finding::indeterminate(
                        "current.served_crl_unreadable",
                        finding.code,
                    )),
                }
            }
        }
    }
    let mut seen: Vec<&[u8]> = Vec::new();
    for subject in subjects {
        if seen.contains(&subject.certificate.der.as_slice()) {
            continue;
        }
        seen.push(&subject.certificate.der);
        findings.push(validity(subject, now));
        if let Some(issuer) = subject.issuer {
            findings.push(revocation(subject, issuer, &served, package_crls, now));
        }
    }
    findings
}

fn validity(subject: &Subject<'_>, now: i64) -> Finding {
    let certificate = subject.certificate;
    if certificate::valid_over(certificate, now, now) {
        Finding::new(Outcome::Checked, "current.within_validity", subject.role)
    } else {
        Finding::failed(
            "current.outside_validity",
            format!(
                "{}: valid {}..={}, now {now}",
                subject.role, certificate.not_before, certificate.not_after
            ),
        )
    }
}

fn revocation(
    subject: &Subject<'_>,
    issuer: &Certificate,
    served: &[Crl],
    package_crls: &[&Crl],
    now: i64,
) -> Finding {
    let chosen = newest(served.iter(), issuer)
        .map(|list| (list, "served CRL"))
        .or_else(|| newest(package_crls.iter().copied(), issuer).map(|list| (list, "package CRL")));
    let Some((list, source)) = chosen else {
        return Finding::indeterminate(
            "current.no_crl",
            format!("{}: no CRL from its issuer", subject.role),
        );
    };
    let finding = crl::status(list, issuer, &subject.certificate.serial, now, now);
    let detail = if finding.detail.is_empty() {
        format!("{} ({source})", subject.role)
    } else {
        format!("{}: {} ({source})", subject.role, finding.detail)
    };
    Finding { detail, ..finding }
}

/// The CRL with the largest number among `lists` that `issuer` issued.
fn newest<'a>(lists: impl Iterator<Item = &'a Crl>, issuer: &Certificate) -> Option<&'a Crl> {
    lists
        .filter(|list| crl::check_issued_by(list, issuer).is_ok())
        .max_by_key(|list| list.number_value())
}

/// Add the time-stamping certificate and its root to the current status.
pub(super) fn tsa_subjects<'a>(tsa: Option<&'a TsaChain>, subjects: &mut Vec<Subject<'a>>) {
    if let Some(tsa) = tsa {
        subjects.push(Subject {
            role: "time-stamping certificate",
            certificate: &tsa.signer,
            issuer: Some(&tsa.root),
        });
        subjects.push(Subject {
            role: "time-stamping root",
            certificate: &tsa.root,
            issuer: None,
        });
    }
}
