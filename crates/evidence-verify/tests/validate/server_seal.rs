//! Complete server seal packages.

use evidence_verify::report::{ComponentKind, InstantSource, Profile};
use evidence_verify::{validate, Outcome};

use crate::support::{assert_complete, component, facts, fixture, policy, Package, DOCUMENT};

#[test]
fn complete_server_seal_with_an_ess_v1_token_is_checked() {
    let report = validate(
        &Package::server_seal(fixture!("token-ess-v1.tsr")).zip(),
        &policy(),
    );
    assert_eq!(report.profile, Some(Profile::ServerSealV1));
    assert_complete(&report);
    let kinds: Vec<ComponentKind> = report.components.iter().map(|c| c.kind).collect();
    assert_eq!(kinds, ComponentKind::ALL.to_vec());
    for kind in [ComponentKind::BindingConsent, ComponentKind::Manifest] {
        assert_eq!(component(&report, kind).outcome, Outcome::NotApplicable);
    }
}

#[test]
fn complete_server_seal_with_an_ess_v2_token_is_checked() {
    let report = validate(
        &Package::server_seal(fixture!("token-ess-v2.tsr")).zip(),
        &policy(),
    );
    assert_complete(&report);
}

#[test]
fn a_bare_token_is_admitted() {
    let report = validate(
        &Package::server_seal(fixture!("token-bare-v2.der")).zip(),
        &policy(),
    );
    assert_complete(&report);
}

#[test]
fn a_server_seal_without_tsa_chain_anchors_the_token_in_ca_pem() {
    let mut package = Package::server_seal(fixture!("token-ess-v1.tsr"));
    package.remove("tsa-chain.pem");
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.profile, Some(Profile::ServerSealV1));
    assert_complete(&report);
}

#[test]
fn instructions_are_optional() {
    let mut package = Package::server_seal(fixture!("token-ess-v1.tsr"));
    package.remove("INSTRUCCIONES.md");
    assert_complete(&validate(&package.zip(), &policy()));
}

#[test]
fn a_server_seal_document_may_carry_an_individual_entry_name() {
    let mut package = Package::server_seal(fixture!("token-ess-v1.tsr"));
    for (from, to) in [
        (DOCUMENT, "binding-root.der"),
        ("acta.pdf.sig", "binding-root.der.sig"),
        ("acta.pdf.tsr", "binding-root.der.tsr"),
    ] {
        let data = package.get(from).to_vec();
        package.remove(from).set(to, &data);
    }
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.profile, Some(Profile::ServerSealV1));
    assert_complete(&report);
}

#[test]
fn the_report_lists_instants_digests_and_the_current_status() {
    let facts = facts();
    let report = validate(
        &Package::server_seal(fixture!("token-ess-v1.tsr")).zip(),
        &policy(),
    );
    let token = report
        .instants
        .iter()
        .find(|instant| instant.source == InstantSource::Token)
        .expect("token instant");
    assert_eq!(token.unix, facts.gen_time);
    assert!(report
        .instants
        .iter()
        .any(|instant| instant.source == InstantSource::Caller
            && instant.unix == facts.gen_time + 3600));
    let document = report
        .digests
        .iter()
        .find(|(name, _)| name == DOCUMENT)
        .expect("document digest");
    assert_eq!(document.1, facts.document_sha256);
    assert_eq!(report.digests.len(), 8);
    assert!(!report.current_status.is_empty());
    assert!(report
        .current_status
        .iter()
        .all(|finding| finding.outcome != Outcome::Failed));
}

#[test]
fn a_certificate_expired_now_changes_only_the_current_status() {
    let facts = facts();
    let mut later = policy();
    later.current_instant = facts.certificate_not_after + 1;
    let report = validate(
        &Package::server_seal(fixture!("token-ess-v1.tsr")).zip(),
        &later,
    );
    assert_complete(&report);
    assert!(report
        .current_status
        .iter()
        .any(|finding| finding.code == "current.outside_validity"));
}
