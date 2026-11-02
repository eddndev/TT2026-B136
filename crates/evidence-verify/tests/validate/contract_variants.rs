//! Bilateral contract packages built from the variant ceremonies of
//! `tests/contract-fixtures/variants`: each one keeps the base entries and
//! replaces only what its ceremony produced differently.

use crate::contract_support::{
    contract_fixture, failed_codes, full, partial, policy, variant_file, with_variant, PARTY_2,
};
use crate::support::{assert_complete, assert_component};
use evidence_verify::policy::{Purposes, ServedRoot, ServedTrust};
use evidence_verify::report::{ComponentKind, Profile};
use evidence_verify::{pem, validate, Outcome, Report};

fn report(package: &crate::support::Package) -> Report {
    validate(&package.zip(), &policy())
}

fn reason_codes(report: &Report) -> Vec<&'static str> {
    report
        .reasons()
        .iter()
        .map(|finding| finding.code)
        .collect()
}

#[test]
fn a_crl_over_64_kib_is_admitted_and_judged() {
    let large = variant_file("large-crl", "parte-1-crl.pem");
    assert!(large.len() > 64 * 1024, "{} bytes", large.len());
    let full_report = report(&with_variant(full(), "large-crl"));
    assert_complete(&full_report);
    let mut package = partial();
    package.set("parte-1-crl.pem", &large);
    let partial_report = report(&package);
    assert_eq!(partial_report.overall, Outcome::Indeterminate);
    assert_eq!(reason_codes(&partial_report), ["contract.partial"]);
}

#[test]
fn a_client_revoked_after_completion_keeps_the_historical_result() {
    let root = pem::decode_one(&contract_fixture("ca.pem"), "CERTIFICATE").expect("root");
    let mut served = policy();
    served.served = Some(ServedTrust {
        roots: vec![ServedRoot {
            der: root,
            purposes: Purposes::ALL,
        }],
        crls: vec![variant_file("revoked-after-completion", "crl-actual.der")],
    });
    let result = validate(&full().zip(), &served);
    assert_complete(&result);
    assert!(
        result.current_status.iter().any(|finding| {
            finding.code == "crl.revoked"
                && finding.detail.starts_with("Client:")
                && finding.detail.ends_with("(served CRL)")
        }),
        "{:#?}",
        result.current_status
    );
}

#[test]
fn a_statement_over_another_request_is_a_statement_mismatch() {
    let mut package = full();
    for entry in ["parte-1-declaracion.bin", "parte-1-declaracion.sig"] {
        package.set(entry, &variant_file("other-request", entry));
    }
    let result = report(&package);
    assert_eq!(result.overall, Outcome::Failed);
    let codes = failed_codes(&result);
    assert!(codes.contains(&"contract.statement_mismatch"), "{codes:?}");
    assert!(!codes.contains(&"signature.invalid"), "{codes:?}");
}

#[test]
fn a_t1_from_a_foreign_authority_is_never_complete() {
    let result = report(&with_variant(full(), "foreign-t1"));
    assert_eq!(result.overall, Outcome::Failed);
    let codes = failed_codes(&result);
    assert!(codes.contains(&"contract.chain_mismatch"), "{codes:?}");
    assert_component(
        &result,
        ComponentKind::DocumentSignature,
        Outcome::NotSupported,
        "token.anchor_not_pinned",
    );
}

#[test]
fn a_tf_after_the_request_expiry_fails_only_that_rule() {
    let result = report(&with_variant(full(), "expired"));
    assert_eq!(failed_codes(&result), ["contract.after_expiry"]);
    assert_component(
        &result,
        ComponentKind::Context,
        Outcome::Failed,
        "contract.after_expiry",
    );
}

#[test]
fn a_t1_before_the_request_creation_fails_only_that_rule() {
    let result = report(&with_variant(partial(), "before-issue"));
    assert_eq!(failed_codes(&result), ["contract.before_issue"]);
}

#[test]
fn two_parties_with_one_key_fail_only_that_rule() {
    let result = report(&with_variant(full(), "same-key"));
    assert_eq!(failed_codes(&result), ["contract.same_key"]);
    assert_component(
        &result,
        ComponentKind::BindingConsent,
        Outcome::Failed,
        "contract.same_key",
    );
}

#[test]
fn a_firm_statement_naming_the_client_account_fails() {
    let result = report(&with_variant(full(), "same-account"));
    let codes = failed_codes(&result);
    assert!(codes.contains(&"contract.same_account"), "{codes:?}");
    assert!(codes.contains(&"contract.party_mismatch"), "{codes:?}");
    assert!(!codes.contains(&"signature.invalid"), "{codes:?}");
}

#[test]
fn a_document_token_older_than_t1_is_out_of_order() {
    let result = report(&with_variant(full(), "inverted-tokens"));
    assert_eq!(failed_codes(&result), ["contract.token_order"]);
    assert_component(
        &result,
        ComponentKind::Context,
        Outcome::Failed,
        "contract.token_order",
    );
}

#[test]
fn a_partial_whose_acknowledgement_is_renamed_as_a_receipt_is_not_supported() {
    let mut package = partial();
    let acknowledgement = package.get("contrato-acuse-parcial.json").to_vec();
    package.remove("contrato-acuse-parcial.json");
    package.set("contrato-recibo.json", &acknowledgement);
    let result = report(&package);
    assert_eq!(result.overall, Outcome::NotSupported);
    assert_ne!(result.profile, Some(Profile::BilateralContractPartialV1));
    assert_ne!(result.profile, Some(Profile::BilateralContractV1));
}

#[test]
fn a_full_package_without_party_2_is_neither_complete_nor_partial() {
    let mut package = full();
    for entry in PARTY_2 {
        package.remove(entry);
    }
    let result = report(&package);
    assert_eq!(result.overall, Outcome::NotSupported);
    assert_component(
        &result,
        ComponentKind::Package,
        Outcome::NotSupported,
        "profile.missing_entry",
    );
}

#[test]
fn a_changed_document_version_breaks_both_statements_and_the_manifest() {
    let mut package = full();
    let mut request = package.get("contrato-solicitud.bin").to_vec();
    // Bytes 74..78 hold the document version, big endian.
    request[77] ^= 0x01;
    package.set("contrato-solicitud.bin", &request);
    let codes = failed_codes(&report(&package));
    assert!(codes.contains(&"contract.statement_mismatch"), "{codes:?}");
    assert!(codes.contains(&"manifest.document_mismatch"), "{codes:?}");
}
