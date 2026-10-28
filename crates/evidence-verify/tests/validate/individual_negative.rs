//! Individual signing packages with one defect each.

use evidence_verify::report::ComponentKind::{
    BindingConsent, Certificates, Context, DocumentSignature, Revocation,
};
use evidence_verify::statements::{decode_consent, encode_consent};
use evidence_verify::{validate, Outcome};

use crate::individual::{complete, instants, rewrite_json, server_json, with_instants};
use crate::support::{altered, assert_component, component, facts, fixture, policy};

#[test]
fn a_consent_for_another_document_fails_binding_and_consent() {
    let (registered, accepted) = instants();
    let mut package = complete();
    let mut consent = decode_consent(package.get("document-consent.bin")).expect("fixture");
    consent.document_sha256[0] ^= 0x01;
    package.set("document-consent.bin", &encode_consent(&consent));
    rewrite_json(&mut package, registered, accepted);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        BindingConsent,
        Outcome::Failed,
        "consent.document_mismatch",
    );
    assert_component(
        &report,
        BindingConsent,
        Outcome::Failed,
        "consent.signature_invalid",
    );
    assert_eq!(
        component(&report, DocumentSignature).outcome,
        Outcome::Checked
    );
}

#[test]
fn a_consent_over_another_document_signature_fails() {
    let mut package = complete();
    package.set("acta.pdf.sig", fixture!("server-signer.sig"));
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        BindingConsent,
        Outcome::Failed,
        "consent.signature_mismatch",
    );
    assert_component(
        &report,
        DocumentSignature,
        Outcome::Failed,
        "signature.invalid",
    );
}

#[test]
fn a_json_record_that_contradicts_the_entries_fails() {
    let (registered, accepted) = instants();
    let mut package = complete();
    let json = server_json(&package, registered, accepted, |key, bytes| {
        if key == "consent_signature" {
            bytes[0] ^= 0x01;
        }
    });
    package.set("individual-signing.json", &json);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        BindingConsent,
        Outcome::Failed,
        "individual.json_mismatch",
    );
}

#[test]
fn an_unreadable_json_record_fails_and_leaves_its_instants_unknown() {
    let mut package = complete();
    package.set("individual-signing.json", b"{");
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        BindingConsent,
        Outcome::Failed,
        "individual_json.malformed",
    );
    assert_component(
        &report,
        Context,
        Outcome::Indeterminate,
        "context.records_unavailable",
    );
}

#[test]
fn a_registration_before_the_binding_crl_is_partial() {
    let facts = facts();
    let (_, accepted) = instants();
    let package = with_instants(facts.crl_this_update - 100, accepted);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Indeterminate, "{report:#?}");
    assert_component(
        &report,
        Revocation,
        Outcome::Indeterminate,
        "crl.not_covering",
    );
}

#[test]
fn an_acceptance_after_the_crl_expired_is_partial() {
    let facts = facts();
    let (registered, _) = instants();
    let package = with_instants(registered, facts.crl_next_update + 100);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Indeterminate, "{report:#?}");
    assert_component(
        &report,
        Revocation,
        Outcome::Indeterminate,
        "crl.not_covering",
    );
}

#[test]
fn an_acceptance_after_the_certificate_expired_fails() {
    let facts = facts();
    let (registered, _) = instants();
    let package = with_instants(registered, facts.certificate_not_after + 10);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        Certificates,
        Outcome::Failed,
        "cert.outside_validity",
    );
}

#[test]
fn a_revoked_holder_fails_revocation() {
    let mut package = complete();
    package.set("certificado.pem", fixture!("revoked-holder.pem"));
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(&report, Revocation, Outcome::Failed, "crl.revoked");
}

#[test]
fn a_holder_with_the_seal_role_fails_the_profile() {
    let mut package = complete();
    package.set("certificado.pem", fixture!("server-signer.pem"));
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        Certificates,
        Outcome::Failed,
        "cert.profile.extensions",
    );
}

#[test]
fn a_binding_root_other_than_ca_pem_fails_the_context() {
    let (registered, accepted) = instants();
    let mut package = complete();
    package.set("binding-root.der", fixture!("foreign-root.der"));
    rewrite_json(&mut package, registered, accepted);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(&report, Context, Outcome::Failed, "context.root_mismatch");
}

#[test]
fn a_truncated_binding_root_fails() {
    let (registered, accepted) = instants();
    let mut package = complete();
    let root = fixture!("root.der");
    package.set("binding-root.der", &root[..root.len() - 1]);
    rewrite_json(&mut package, registered, accepted);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(&report, Certificates, Outcome::Failed, "cert.malformed");
}

#[test]
fn different_crls_for_the_same_trust_revision_fail_the_context() {
    let mut package = complete();
    package.set("crl.pem", fixture!("root-empty.crl.pem"));
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(&report, Context, Outcome::Failed, "context.trust_mismatch");
}

#[test]
fn a_registration_after_the_acceptance_fails() {
    let (_, accepted) = instants();
    let report = validate(&with_instants(accepted + 1, accepted).zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        Context,
        Outcome::Failed,
        "context.registration_after_acceptance",
    );
}

#[test]
fn a_token_after_the_acceptance_beyond_its_accuracy_fails() {
    let facts = facts();
    let (registered, _) = instants();
    let accepted = facts.gen_time - facts.accuracy - 1;
    let report = validate(&with_instants(registered, accepted).zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        Context,
        Outcome::Failed,
        "context.token_after_acceptance",
    );
}

#[test]
fn an_altered_binding_signature_fails() {
    let (registered, accepted) = instants();
    let mut package = complete();
    let signature = altered(package.get("certificate-binding.sig"));
    package.set("certificate-binding.sig", &signature);
    rewrite_json(&mut package, registered, accepted);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        BindingConsent,
        Outcome::Failed,
        "binding.signature_invalid",
    );
}
