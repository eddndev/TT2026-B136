//! Individual signing v2 packages with one defect each. Changing a covered
//! entry also changes its manifest digest, so those cases assert the
//! component that names the defect rather than the only reason.

use evidence_verify::manifest::Role;
use evidence_verify::report::ComponentKind::{Context, DocumentToken, Manifest, Revocation};
use evidence_verify::report::Report;
use evidence_verify::{validate, Outcome};

use crate::individual::{instants, with_instants};
use crate::individual_v2::{complete, edit_manifest, receipt_of, set_receipt, v2_from};
use crate::support::{altered, assert_component, component, facts, fixture, policy, Package};

fn report(package: &Package) -> Report {
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed, "{report:#?}");
    report
}

fn manifest_fails(package: &Package, code: &str) -> Report {
    let report = report(package);
    assert_component(&report, Manifest, Outcome::Failed, code);
    report
}

#[test]
fn a_flipped_manifest_byte_fails_the_manifest() {
    let mut package = complete();
    let flipped = altered(package.get("manifiesto.json"));
    package.set("manifiesto.json", &flipped);
    manifest_fails(&package, "json.unexpected_byte");
    let text = String::from_utf8(fixture!("manifiesto.json").to_vec()).expect("ASCII");
    let digest = facts().document_sha256;
    let first = if digest.starts_with('0') { "1" } else { "0" };
    let other = text.replacen(&digest, &format!("{first}{}", &digest[1..]), 1);
    assert_ne!(other, text);
    package.set("manifiesto.json", other.as_bytes());
    let report = manifest_fails(&package, "token.imprint_mismatch");
    assert_component(
        &report,
        Manifest,
        Outcome::Failed,
        "manifest.artifact_mismatch",
    );
    assert_component(
        &report,
        Context,
        Outcome::Failed,
        "receipt.manifest_mismatch",
    );
}

#[test]
fn reordered_manifest_keys_are_not_canonical() {
    let mut package = complete();
    let text = String::from_utf8(fixture!("manifiesto.json").to_vec()).expect("ASCII");
    let reordered = text.replacen(
        r#""digest":"sha-256","signature":"rsa-pkcs1v15-sha256""#,
        r#""signature":"rsa-pkcs1v15-sha256","digest":"sha-256""#,
        1,
    );
    assert_ne!(reordered, text);
    package.set("manifiesto.json", reordered.as_bytes());
    manifest_fails(&package, "json.not_canonical");
}

#[test]
fn swapped_roles_fail_the_manifest() {
    let mut package = complete();
    edit_manifest(&mut package, |manifest| {
        for artifact in &mut manifest.artifacts {
            artifact.role = match artifact.role {
                Role::BindingStatement => Role::ConsentStatement,
                Role::ConsentStatement => Role::BindingStatement,
                other => other,
            };
        }
    });
    let report = manifest_fails(&package, "manifest.role_mismatch");
    assert_component(&report, Manifest, Outcome::Failed, "token.imprint_mismatch");
}

#[test]
fn a_token_over_another_object_fails_the_manifest_token() {
    for token in [
        fixture!("token-ess-v1.tsr"),
        fixture!("token-other-digest.tsr"),
    ] {
        let mut package = complete();
        package.set("manifiesto.json.tsr", token);
        let report = manifest_fails(&package, "token.imprint_mismatch");
        assert_eq!(component(&report, DocumentToken).outcome, Outcome::Checked);
    }
}

#[test]
fn a_manifest_row_removed_while_the_entry_stays_is_missing() {
    let mut package = complete();
    edit_manifest(&mut package, |manifest| {
        manifest
            .artifacts
            .retain(|artifact| artifact.role != Role::BindingCrl);
    });
    manifest_fails(&package, "manifest.artifact_missing");
}

#[test]
fn an_artifact_that_is_not_a_covered_entry_is_unexpected() {
    let mut package = complete();
    edit_manifest(&mut package, |manifest| {
        for artifact in &mut manifest.artifacts {
            if artifact.role == Role::BindingCrl {
                artifact.entry = "individual-signing.json".to_owned();
            }
        }
    });
    let report = manifest_fails(&package, "manifest.artifact_unexpected");
    assert_component(&report, Manifest, Outcome::Failed, "manifest.role_mismatch");
}

#[test]
fn a_document_digest_or_identity_that_contradicts_the_consent_fails() {
    let mut package = complete();
    edit_manifest(&mut package, |manifest| manifest.document_sha256[0] ^= 0x01);
    manifest_fails(&package, "manifest.document_mismatch");
    let mut package = complete();
    edit_manifest(&mut package, |manifest| {
        manifest.case_id = "00000000-0000-4000-8000-000000000000".to_owned();
    });
    manifest_fails(&package, "manifest.document_mismatch");
    let mut package = complete();
    edit_manifest(&mut package, |manifest| {
        manifest.signers[0].account_id = "00000000-0000-4000-8000-000000000000".to_owned();
    });
    manifest_fails(&package, "manifest.signer_mismatch");
    let mut package = complete();
    edit_manifest(&mut package, |manifest| {
        manifest.signers[0].entries.binding_root = "binding-crl.der".to_owned();
    });
    manifest_fails(&package, "manifest.signer_mismatch");
}

#[test]
fn a_tsa_crl_from_another_issuer_fails_revocation() {
    let mut package = complete();
    package.set("tsa-crl.pem", fixture!("foreign.crl.pem"));
    let report = manifest_fails(&package, "manifest.artifact_mismatch");
    assert_component(
        &report,
        Revocation,
        Outcome::Failed,
        "crl.issuer.key_id_mismatch",
    );
}

#[test]
fn a_stale_tsa_crl_leaves_the_manifest_token_revocation_indeterminate() {
    let mut package = complete();
    package.set("tsa-crl.pem", fixture!("root-stale.crl.pem"));
    let report = manifest_fails(&package, "manifest.artifact_mismatch");
    assert_component(
        &report,
        Revocation,
        Outcome::Indeterminate,
        "crl.not_covering",
    );
}

#[test]
fn a_tsa_crl_that_revokes_the_time_stamping_certificate_fails_revocation() {
    let mut package = complete();
    package.set("tsa-crl.pem", fixture!("root-tsa-revoked.crl.pem"));
    let report = manifest_fails(&package, "manifest.artifact_mismatch");
    assert_component(&report, Revocation, Outcome::Failed, "crl.revoked");
    let revoked = component(&report, Revocation)
        .findings
        .iter()
        .filter(|finding| finding.code == "crl.revoked")
        .count();
    assert_eq!(revoked, 2, "both tokens: {report:#?}");
}

#[test]
fn a_receipt_that_contradicts_the_package_fails_the_context() {
    let (_, accepted) = instants();
    let base = complete();
    let good = receipt_of(&base, accepted);
    let mut seconds = good.clone();
    seconds.accepted_at_seconds += 1;
    let mut digest = good.clone();
    digest.manifest_sha256[0] ^= 0x01;
    let mut operation = good.clone();
    operation.operation_id = "00000000-0000-4000-8000-000000000000".to_owned();
    let cases = [
        ("receipt.seconds_mismatch", seconds),
        ("receipt.manifest_mismatch", digest),
        ("receipt.operation_mismatch", operation),
    ];
    for (code, receipt) in cases {
        let mut package = base.clone();
        set_receipt(&mut package, &receipt);
        let report = report(&package);
        assert_component(&report, Context, Outcome::Failed, code);
        assert_eq!(component(&report, Manifest).outcome, Outcome::Checked);
    }
    let mut package = base.clone();
    package.set("recibo-aceptacion.json", b"{}");
    assert_component(
        &report(&package),
        Context,
        Outcome::Failed,
        "receipt.schema",
    );
}

#[test]
fn a_document_token_after_the_manifest_token_fails_the_context() {
    let mut package = complete();
    package.set("acta.pdf.tsr", fixture!("token-after-manifest.tsr"));
    let report = manifest_fails(&package, "manifest.artifact_mismatch");
    assert_component(
        &report,
        Context,
        Outcome::Failed,
        "context.document_after_manifest",
    );
}

#[test]
fn an_acceptance_before_the_manifest_token_fails_the_context() {
    let (registered, _) = instants();
    let accepted = facts().gen_time - facts().accuracy - 5;
    let package = v2_from(with_instants(registered, accepted), accepted);
    let report = report(&package);
    assert_component(
        &report,
        Context,
        Outcome::Failed,
        "context.manifest_after_acceptance",
    );
}

#[test]
fn a_registration_instant_that_differs_from_the_manifest_fails_the_context() {
    let (registered, accepted) = instants();
    let package = v2_from(with_instants(registered + 1, accepted), accepted);
    let report = report(&package);
    assert_component(
        &report,
        Context,
        Outcome::Failed,
        "context.binding_registration_mismatch",
    );
    assert_eq!(component(&report, Manifest).outcome, Outcome::Checked);
}

#[test]
fn a_revocation_in_crl_pem_fails_the_manifest_token_too() {
    let mut package = complete();
    package.set("crl.pem", fixture!("root-tsa-revoked.crl.pem"));
    let report = manifest_fails(&package, "manifest.artifact_mismatch");
    let revoked: Vec<&str> = component(&report, Revocation)
        .findings
        .iter()
        .filter(|finding| finding.code == "crl.revoked")
        .map(|finding| finding.detail.as_str())
        .collect();
    assert!(
        revoked
            .iter()
            .any(|detail| detail.starts_with("manifest token time-stamping certificate (crl.pem)")),
        "{report:#?}"
    );
    assert!(
        revoked
            .iter()
            .any(|detail| detail.starts_with("document token time-stamping certificate (crl.pem)")),
        "{report:#?}"
    );
}
