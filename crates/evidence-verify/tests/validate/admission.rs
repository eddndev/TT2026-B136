//! Admission and profile recognition through the validation entry point.

use evidence_verify::report::{ComponentKind, Profile};
use evidence_verify::{validate, Outcome};

use crate::support::{assert_component, fixture, policy, zip, Package};

fn seal() -> Package {
    Package::server_seal(fixture!("token-ess-v1.tsr"))
}

fn refused(package: &Package, outcome: Outcome, code: &str) {
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.profile, None, "{report:#?}");
    assert_eq!(report.overall, outcome, "{report:#?}");
    assert_eq!(report.components.len(), 1);
    assert_component(&report, ComponentKind::Package, outcome, code);
}

#[test]
fn bytes_that_are_not_a_package_are_not_supported() {
    let report = validate(b"not a zip", &policy());
    assert_eq!(report.overall, Outcome::NotSupported);
    assert_eq!(report.profile, None);
    assert!(report.digests.is_empty());
}

#[test]
fn a_corrupted_entry_fails_the_package() {
    let mut bytes = seal().zip();
    bytes[40] ^= 0x01;
    let report = validate(&bytes, &policy());
    assert_eq!(report.overall, Outcome::Failed);
    assert_component(
        &report,
        ComponentKind::Package,
        Outcome::Failed,
        "zip.crc_mismatch",
    );
}

#[test]
fn an_oversized_entry_is_not_supported() {
    let mut package = seal();
    package.set("ca.pem", &vec![b'A'; 64 * 1024 + 1]);
    refused(&package, Outcome::NotSupported, "zip.entry_too_large");
}

#[test]
fn two_documents_make_the_package_ambiguous() {
    let mut package = seal();
    package
        .set("otro.pdf", fixture!("document.bin"))
        .set("otro.pdf.sig", fixture!("server-signer.sig"))
        .set("otro.pdf.tsr", fixture!("token-ess-v1.tsr"));
    refused(
        &package,
        Outcome::NotSupported,
        "profile.ambiguous_document",
    );
}

#[test]
fn an_unknown_entry_is_not_supported() {
    let mut package = seal();
    package.set("notas.txt", b"x");
    refused(&package, Outcome::NotSupported, "profile.unknown_entry");
}

#[test]
fn a_missing_required_entry_is_not_supported() {
    let mut package = seal();
    package.remove("crl.pem");
    refused(&package, Outcome::NotSupported, "profile.missing_entry");
}

#[test]
fn a_document_without_its_token_is_not_supported() {
    let mut package = seal();
    package.remove("acta.pdf.tsr");
    refused(&package, Outcome::NotSupported, "profile.no_document");
}

#[test]
fn names_are_compared_byte_for_byte() {
    let mut package = seal();
    let data = package.get("ca.pem").to_vec();
    package.remove("ca.pem").set("CA.pem", &data);
    refused(&package, Outcome::NotSupported, "profile.missing_entry");
}

#[test]
fn an_individual_package_missing_one_statement_entry_is_not_supported() {
    let mut package = crate::individual::complete();
    package.remove("binding-crl.der");
    refused(&package, Outcome::NotSupported, "profile.missing_entry");
}

#[test]
fn an_individual_package_requires_the_tsa_chain() {
    let mut package = crate::individual::complete();
    package.remove("tsa-chain.pem");
    refused(&package, Outcome::NotSupported, "profile.missing_entry");
}

#[test]
fn a_manifest_package_is_recognized_and_not_supported_yet() {
    let mut package = crate::individual::complete();
    package
        .set("manifiesto.json", b"{}")
        .set("manifiesto.json.tsr", fixture!("token-ess-v1.tsr"))
        .set("tsa-crl.pem", fixture!("root.crl.pem"))
        .set("recibo-aceptacion.json", b"{}");
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.profile, Some(Profile::IndividualSigningV2));
    assert_eq!(report.overall, Outcome::NotSupported);
    assert_component(
        &report,
        ComponentKind::Manifest,
        Outcome::NotSupported,
        "profile.manifest_not_available",
    );
    assert_eq!(report.components.len(), ComponentKind::ALL.len());
}

#[test]
fn the_server_writer_layout_is_reproduced_by_the_test_writer() {
    let bytes = zip(&[("a.txt", b"x")]);
    assert_eq!(&bytes[..4], b"PK\x03\x04");
    assert!(evidence_verify::archive::admit(&bytes).is_ok());
}
