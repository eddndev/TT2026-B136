//! Server seal packages with one defect each, and the component and overall
//! outcome the contract assigns to it.

use evidence_verify::report::ComponentKind::{
    Certificates, DocumentSignature, DocumentToken, Revocation,
};
use evidence_verify::{validate, Outcome};

use crate::support::{assert_component, component, fixture, policy, Package, DOCUMENT};

fn seal() -> Package {
    Package::server_seal(fixture!("token-ess-v1.tsr"))
}

#[test]
fn an_altered_document_fails_the_token_and_the_signature() {
    let mut package = seal();
    let mut document = package.get(DOCUMENT).to_vec();
    document[0] ^= 0x20;
    package.set(DOCUMENT, &document);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        DocumentToken,
        Outcome::Failed,
        "token.imprint_mismatch",
    );
    assert_component(
        &report,
        DocumentSignature,
        Outcome::Failed,
        "signature.invalid",
    );
    // The profiles still hold; only the instant of the validity windows is
    // missing, since the failed token establishes no genTime.
    assert_component(
        &report,
        Certificates,
        Outcome::Indeterminate,
        "cert.instant_unknown",
    );
    assert!(component(&report, Certificates)
        .findings
        .iter()
        .all(|finding| finding.outcome != Outcome::Failed));
}

#[test]
fn an_altered_signature_fails_only_the_signature() {
    let mut package = seal();
    let signature = crate::support::altered(package.get("acta.pdf.sig"));
    package.set("acta.pdf.sig", &signature);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        DocumentSignature,
        Outcome::Failed,
        "signature.invalid",
    );
    assert_eq!(component(&report, DocumentToken).outcome, Outcome::Checked);
}

#[test]
fn a_token_over_another_digest_fails() {
    let report = validate(
        &Package::server_seal(fixture!("token-other-digest.tsr")).zip(),
        &policy(),
    );
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        DocumentToken,
        Outcome::Failed,
        "token.imprint_mismatch",
    );
    assert_eq!(
        component(&report, DocumentSignature).outcome,
        Outcome::Checked
    );
}

#[test]
fn a_truncated_token_is_failed() {
    let token = fixture!("token-ess-v1.tsr");
    let report = validate(
        &Package::server_seal(&token[..token.len() - 7]).zip(),
        &policy(),
    );
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(&report, DocumentToken, Outcome::Failed, "token.malformed");
}

#[test]
fn a_token_with_a_policy_outside_the_admitted_list_is_not_supported() {
    let report = validate(
        &Package::server_seal(fixture!("token-other-policy.tsr")).zip(),
        &policy(),
    );
    assert_eq!(report.overall, Outcome::NotSupported);
    assert_component(
        &report,
        DocumentToken,
        Outcome::NotSupported,
        "token.policy_not_admitted",
    );
}

#[test]
fn the_caller_policy_list_decides_which_token_policies_are_admitted() {
    let mut strict = policy();
    strict.admitted_policies = vec!["1.3.6.1.4.1.13762.4".to_owned()];
    let report = validate(&seal().zip(), &strict);
    assert_eq!(report.overall, Outcome::NotSupported);
    assert_component(
        &report,
        DocumentToken,
        Outcome::NotSupported,
        "token.policy_not_admitted",
    );
}

#[test]
fn a_token_from_a_foreign_authority_is_not_supported_even_with_its_root_in_the_zip() {
    let mut package = Package::server_seal(fixture!("token-foreign.tsr"));
    package.set("tsa-chain.pem", fixture!("foreign-root.pem"));
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::NotSupported);
    assert_component(
        &report,
        DocumentToken,
        Outcome::NotSupported,
        "token.anchor_not_pinned",
    );
}

#[test]
fn a_signer_with_another_role_fails_the_profile_but_not_the_signature() {
    let mut package = seal();
    package
        .set("certificado.pem", fixture!("holder.pem"))
        .set("acta.pdf.sig", fixture!("holder.sig"));
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        Certificates,
        Outcome::Failed,
        "cert.profile.extensions",
    );
    assert_eq!(
        component(&report, DocumentSignature).outcome,
        Outcome::Checked
    );
    assert_eq!(component(&report, DocumentToken).outcome, Outcome::Checked);
}

#[test]
fn a_crl_of_another_issuer_fails_revocation() {
    let mut package = seal();
    package.set("crl.pem", fixture!("foreign.crl.pem"));
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        Revocation,
        Outcome::Failed,
        "crl.issuer.key_id_mismatch",
    );
}

#[test]
fn an_unreadable_crl_fails_revocation() {
    let mut package = seal();
    package.set("crl.pem", b"");
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(&report, Revocation, Outcome::Failed, "pem.no_block");
}

#[test]
fn an_unreadable_signer_certificate_fails_certificates() {
    let mut package = seal();
    package.set(
        "certificado.pem",
        b"-----BEGIN CERTIFICATE-----\nMAA=\n-----END CERTIFICATE-----\n",
    );
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(&report, Certificates, Outcome::Failed, "cert.malformed");
    assert_component(
        &report,
        DocumentSignature,
        Outcome::Indeterminate,
        "signature.key_unavailable",
    );
}

#[test]
fn a_tsa_chain_beyond_the_certificate_limit_is_not_supported() {
    let mut package = seal();
    package.set("tsa-chain.pem", &fixture!("root.pem").repeat(9));
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::NotSupported, "{report:#?}");
    assert_component(
        &report,
        DocumentToken,
        Outcome::NotSupported,
        "pem.too_many_blocks",
    );
}

#[test]
fn a_tsa_chain_repeating_the_pinned_root_up_to_the_limit_is_admitted() {
    let mut package = seal();
    package.set("tsa-chain.pem", &fixture!("root.pem").repeat(8));
    crate::support::assert_complete(&validate(&package.zip(), &policy()));
}
