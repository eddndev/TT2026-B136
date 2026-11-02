//! Altered bilateral contract packages: each alteration fails or is not
//! supported, with the code that names it.

use crate::contract_support::{contract_fixture, full, partial, policy, PARTY_1, PARTY_2};
use crate::support::{altered, assert_component, Package};
use evidence_verify::report::ComponentKind;
use evidence_verify::{validate, Outcome, Report};

fn report(package: &Package) -> Report {
    validate(&package.zip(), &policy())
}

/// Assert `report` failed and some finding carries `code`.
fn failed_with(report: &Report, code: &str) {
    assert_eq!(report.overall, Outcome::Failed, "{report:#?}");
    let codes: Vec<&str> = report
        .components
        .iter()
        .flat_map(|component| component.findings.iter())
        .filter(|finding| finding.outcome == Outcome::Failed)
        .map(|finding| finding.code)
        .collect();
    assert!(codes.contains(&code), "{code} not in {codes:?}");
}

#[test]
fn one_altered_byte_in_any_interpreted_entry_is_never_complete() {
    let base = full();
    for (name, data) in &base.entries {
        if name == "INSTRUCCIONES.md" {
            continue;
        }
        let mut package = base.clone();
        package.set(name, &altered(data));
        let result = report(&package);
        assert_ne!(result.overall, Outcome::Checked, "{name}");
        assert_ne!(result.overall, Outcome::Indeterminate, "{name}");
    }
}

#[test]
fn removing_a_mandatory_entry_is_not_supported() {
    let base = full();
    for (name, _) in &base.entries {
        if name == "INSTRUCCIONES.md" {
            continue;
        }
        let mut package = base.clone();
        package.remove(name);
        let result = report(&package);
        assert_eq!(result.overall, Outcome::NotSupported, "{name}");
        assert_component(
            &result,
            ComponentKind::Package,
            Outcome::NotSupported,
            "profile.missing_entry",
        );
    }
}

#[test]
fn a_changed_pdf_breaks_the_statements_the_token_and_the_manifest() {
    let mut package = full();
    let pdf = package.get("contrato.pdf").to_vec();
    package.set("contrato.pdf", &altered(&pdf));
    let result = report(&package);
    failed_with(&result, "contract.statement_mismatch");
    failed_with(&result, "token.imprint_mismatch");
    failed_with(&result, "manifest.artifact_mismatch");
}

#[test]
fn an_altered_request_breaks_its_schema_or_both_statements() {
    let mut package = full();
    let mut request = package.get("contrato-solicitud.bin").to_vec();
    request[239] = 2;
    package.set("contrato-solicitud.bin", &request);
    failed_with(&report(&package), "contract.request_schema");
    let mut package = full();
    let mut request = package.get("contrato-solicitud.bin").to_vec();
    request[237] ^= 0x01;
    package.set("contrato-solicitud.bin", &request);
    failed_with(&report(&package), "contract.statement_mismatch");
}

#[test]
fn a_replaced_counterparty_certificate_is_a_party_mismatch() {
    let mut package = full();
    package.set(
        "parte-2-certificado.pem",
        &contract_fixture("parte-1-certificado.pem"),
    );
    let result = report(&package);
    failed_with(&result, "contract.party_mismatch");
    failed_with(&result, "contract.same_key");
    failed_with(&result, "signature.invalid");
}

#[test]
fn a_replaced_t1_breaks_its_imprint_and_the_hash_chain() {
    let mut package = full();
    package.set(
        "parte-1-declaracion.sig.tsr",
        &contract_fixture("contrato.pdf.tsr"),
    );
    let result = report(&package);
    failed_with(&result, "token.imprint_mismatch");
    failed_with(&result, "contract.chain_mismatch");
}

#[test]
fn swapped_parties_fail() {
    let mut package = full();
    for (first, second) in PARTY_1.iter().zip(&PARTY_2) {
        let one = contract_fixture(first);
        let two = contract_fixture(second);
        package.set(first, &two);
        package.set(second, &one);
    }
    let result = report(&package);
    failed_with(&result, "contract.statement_mismatch");
    failed_with(&result, "contract.party_mismatch");
}

#[test]
fn inverted_tokens_fail_their_imprints() {
    let mut package = full();
    package.set(
        "contrato.pdf.tsr",
        &contract_fixture("contrato-manifiesto.json.tsr"),
    );
    package.set(
        "contrato-manifiesto.json.tsr",
        &contract_fixture("contrato.pdf.tsr"),
    );
    failed_with(&report(&package), "token.imprint_mismatch");
}

#[test]
fn a_client_revoked_before_the_counter_signature_fails() {
    let mut package = full();
    package.set(
        "parte-2-crl.pem",
        &contract_fixture("parte-2-crl-cliente-revocado.pem"),
    );
    let result = report(&package);
    failed_with(&result, "contract.counterparty_revoked");
    assert_component(
        &result,
        ComponentKind::Revocation,
        Outcome::Failed,
        "contract.counterparty_revoked",
    );
}

#[test]
fn a_partial_with_the_firm_statement_is_not_a_partial() {
    let mut package = partial();
    package.set(
        "parte-2-declaracion.bin",
        &contract_fixture("parte-2-declaracion.bin"),
    );
    assert_eq!(report(&package).overall, Outcome::NotSupported);
}

#[test]
fn the_receipt_must_have_its_schema() {
    let mut package = full();
    package.set(
        "contrato-recibo.json",
        br#"{"provenance":"application-record"}"#,
    );
    let result = report(&package);
    assert_component(
        &result,
        ComponentKind::Context,
        Outcome::Failed,
        "receipt.contract_schema",
    );
}
