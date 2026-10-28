//! Complete individual signing packages, built from the fixture statements
//! with `individual-signing.json` in the compact form the server writes.

use evidence_verify::report::{ComponentKind, InstantSource, Profile};
use evidence_verify::{validate, Outcome};

use crate::support::{
    assert_complete, assert_component, facts, fixture, policy, Package, DOCUMENT,
};

/// The registration and acceptance instants of [`complete`]: registration
/// just after the fixture CRLs start and acceptance just after the token.
pub fn instants() -> (i64, i64) {
    let facts = facts();
    (facts.crl_this_update + 60, facts.gen_time + 60)
}

/// A complete individual signing export in the order the server writes it.
pub fn complete() -> Package {
    let (registered, accepted) = instants();
    with_instants(registered, accepted)
}

pub fn with_instants(registered: i64, accepted: i64) -> Package {
    let mut package = Package::new(&[
        (DOCUMENT, fixture!("document.bin")),
        ("acta.pdf.sig", fixture!("holder.sig")),
        ("acta.pdf.tsr", fixture!("token-ess-v1.tsr")),
        ("certificado.pem", fixture!("holder.pem")),
        ("ca.pem", fixture!("root.pem")),
        ("crl.pem", fixture!("root.crl.pem")),
        ("tsa-chain.pem", fixture!("root.pem")),
        ("document-consent.bin", fixture!("document-consent.bin")),
        ("document-consent.sig", fixture!("document-consent.sig")),
        (
            "certificate-binding.bin",
            fixture!("certificate-binding.bin"),
        ),
        (
            "certificate-binding.sig",
            fixture!("certificate-binding.sig"),
        ),
        ("binding-root.der", fixture!("root.der")),
        ("binding-crl.der", fixture!("root.crl.der")),
        ("individual-signing.json", b"{}"),
        ("INSTRUCCIONES.md", b"# Instrucciones\n"),
    ]);
    rewrite_json(&mut package, registered, accepted);
    package
}

/// Rewrite `individual-signing.json` from the statement entries of
/// `package`, so that it stays consistent with them.
pub fn rewrite_json(package: &mut Package, registered: i64, accepted: i64) {
    let json = server_json(package, registered, accepted, |_, _| {});
    package.set("individual-signing.json", &json);
}

/// The server's JSON form of the entries of `package`, after `edit` may
/// change one member's bytes.
pub fn server_json(
    package: &Package,
    registered: i64,
    accepted: i64,
    edit: impl Fn(&str, &mut Vec<u8>),
) -> Vec<u8> {
    let array = |key: &str, entry: &str| {
        let mut bytes = package.get(entry).to_vec();
        edit(key, &mut bytes);
        let items: Vec<String> = bytes.iter().map(u8::to_string).collect();
        format!("\"{key}\":[{}]", items.join(","))
    };
    let members = [
        "\"format_version\":1".to_owned(),
        array("binding_statement", "certificate-binding.bin"),
        array("binding_signature", "certificate-binding.sig"),
        array("binding_root_der", "binding-root.der"),
        array("binding_crl_der", "binding-crl.der"),
        format!("\"binding_registered_at\":{registered}"),
        array("consent_statement", "document-consent.bin"),
        array("consent_signature", "document-consent.sig"),
        format!("\"accepted_at\":{accepted}"),
    ];
    format!("{{{}}}", members.join(",")).into_bytes()
}

#[test]
fn complete_individual_signing_is_checked() {
    let report = validate(&complete().zip(), &policy());
    assert_eq!(report.profile, Some(Profile::IndividualSigningV1));
    assert_complete(&report);
    assert_eq!(
        report.components.iter().map(|c| c.kind).collect::<Vec<_>>(),
        ComponentKind::ALL.to_vec()
    );
    assert_eq!(
        crate::support::component(&report, ComponentKind::Manifest).outcome,
        Outcome::NotApplicable
    );
    assert_eq!(
        crate::support::component(&report, ComponentKind::BindingConsent).outcome,
        Outcome::Checked
    );
}

#[test]
fn application_records_are_reported_as_uncovered_instants() {
    let (registered, accepted) = instants();
    let report = validate(&complete().zip(), &policy());
    let records: Vec<i64> = report
        .instants
        .iter()
        .filter(|instant| instant.source == InstantSource::ApplicationRecord)
        .map(|instant| instant.unix)
        .collect();
    assert_eq!(records, vec![registered, accepted]);
    assert_component(
        &report,
        ComponentKind::Context,
        Outcome::Checked,
        "context.records_not_time_stamped",
    );
}

#[test]
fn a_package_with_registration_and_acceptance_at_the_same_instant_is_checked() {
    let (_, accepted) = instants();
    assert_complete(&validate(
        &with_instants(accepted, accepted).zip(),
        &policy(),
    ));
}

#[test]
fn a_token_within_its_accuracy_after_acceptance_is_admitted() {
    let facts = facts();
    let accepted = facts.gen_time - facts.accuracy;
    let report = validate(
        &with_instants(facts.crl_this_update + 60, accepted).zip(),
        &policy(),
    );
    assert_complete(&report);
}
