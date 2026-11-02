//! Complete bilateral contract packages: the full package is checked with a
//! CRL per token, and the partial package is never complete.

use crate::contract_support::{fact, full, partial, policy};
use crate::support::{assert_complete, assert_component, component};
use evidence_verify::report::{ComponentKind, InstantSource, Profile};
use evidence_verify::{validate, Outcome};

fn details(report: &evidence_verify::Report, kind: ComponentKind) -> Vec<String> {
    component(report, kind)
        .findings
        .iter()
        .map(|finding| format!("{} {}", finding.code, finding.detail))
        .collect()
}

#[test]
fn the_full_package_is_complete() {
    let report = validate(&full().zip(), &policy());
    assert_eq!(report.profile, Some(Profile::BilateralContractV1));
    assert_complete(&report);
    assert_eq!(report.components.len(), 8);
    for kind in ComponentKind::ALL {
        assert_eq!(
            component(&report, kind).outcome,
            Outcome::Checked,
            "{kind:?}"
        );
    }
    assert_component(
        &report,
        ComponentKind::DocumentToken,
        Outcome::Checked,
        "token.verified",
    );
    assert_component(
        &report,
        ComponentKind::DocumentSignature,
        Outcome::Checked,
        "signature.valid",
    );
    assert_component(
        &report,
        ComponentKind::Manifest,
        Outcome::Checked,
        "token.verified",
    );
}

#[test]
fn the_full_package_holds_a_pdf_over_64_kib() {
    let package = full();
    assert!(package.get("contrato.pdf").len() > 64 * 1024);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Checked);
}

#[test]
fn the_client_is_judged_at_t1_and_the_firm_signer_at_tf() {
    let report = validate(&full().zip(), &policy());
    let certificates = details(&report, ComponentKind::Certificates);
    // Every fixture token has an accuracy of one second.
    let interval = |at: i64| format!("{}..={}", at - 1, at + 1);
    let t1 = interval(fact(&["instants", "t1"]));
    let tf = interval(fact(&["instants", "tf"]));
    assert!(
        certificates
            .iter()
            .any(|line| line.starts_with("cert.validity T1 genTime") && line.ends_with(&t1)),
        "{certificates:#?}"
    );
    assert!(
        certificates
            .iter()
            .any(|line| line.starts_with("cert.validity Tf genTime") && line.ends_with(&tf)),
        "{certificates:#?}"
    );
    let labels: Vec<(&str, InstantSource)> = report
        .instants
        .iter()
        .map(|instant| (instant.label.as_str(), instant.source))
        .collect();
    for label in ["T1 genTime", "Td genTime", "Tf genTime"] {
        assert!(
            labels.contains(&(label, InstantSource::Token)),
            "{labels:?}"
        );
    }
    assert!(
        labels.contains(&("receipt completed_at", InstantSource::ApplicationRecord)),
        "{labels:?}"
    );
}

#[test]
fn each_token_is_judged_with_the_crl_of_its_own_instant() {
    let report = validate(&full().zip(), &policy());
    let revocation = details(&report, ComponentKind::Revocation);
    assert!(
        revocation
            .iter()
            .all(|line| !line.contains("crl.not_covering")),
        "{revocation:#?}"
    );
    // A single list captured at completion does not cover T1.
    let mut single = full();
    let later = single.get("parte-2-crl.pem").to_vec();
    single.set("parte-1-crl.pem", &later);
    let report = validate(&single.zip(), &policy());
    let revocation = component(&report, ComponentKind::Revocation);
    assert!(
        revocation
            .findings
            .iter()
            .any(|finding| finding.code == "crl.not_covering" && finding.detail.contains("T1")),
        "{revocation:#?}"
    );
}

#[test]
fn the_partial_package_is_never_complete() {
    let report = validate(&partial().zip(), &policy());
    assert_eq!(report.profile, Some(Profile::BilateralContractPartialV1));
    assert_eq!(report.overall, Outcome::Indeterminate, "{report:#?}");
    assert_eq!(report.components.len(), 8);
    for kind in [ComponentKind::DocumentToken, ComponentKind::Manifest] {
        assert_eq!(
            component(&report, kind).outcome,
            Outcome::NotApplicable,
            "{kind:?}"
        );
    }
    for kind in [
        ComponentKind::Package,
        ComponentKind::DocumentSignature,
        ComponentKind::BindingConsent,
        ComponentKind::Certificates,
        ComponentKind::Revocation,
    ] {
        assert_eq!(
            component(&report, kind).outcome,
            Outcome::Checked,
            "{kind:?}"
        );
    }
    assert_component(
        &report,
        ComponentKind::Context,
        Outcome::Indeterminate,
        "contract.partial",
    );
    let reasons: Vec<&str> = report
        .reasons()
        .iter()
        .map(|finding| finding.code)
        .collect();
    assert_eq!(reasons, ["contract.partial"]);
}

#[test]
fn the_partial_package_judges_the_binding_at_t1() {
    let report = validate(&partial().zip(), &policy());
    let certificates = details(&report, ComponentKind::Certificates);
    assert!(
        certificates
            .iter()
            .all(|line| !line.contains("binding_registered_at")),
        "{certificates:#?}"
    );
    let revocation = details(&report, ComponentKind::Revocation);
    assert!(
        revocation
            .iter()
            .any(|line| line.starts_with("crl.not_revoked Client binding at T1 genTime")),
        "{revocation:#?}"
    );
}

#[test]
fn the_acknowledgement_decides_nothing_but_must_have_its_schema() {
    let mut package = partial();
    package.set(
        "contrato-acuse-parcial.json",
        br#"{"provenance":"application-record"}"#,
    );
    let report = validate(&package.zip(), &policy());
    assert_component(
        &report,
        ComponentKind::Context,
        Outcome::Failed,
        "receipt.contract_partial_schema",
    );
}
