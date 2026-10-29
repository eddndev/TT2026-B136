//! Complete individual signing v2 packages: the v1 package of
//! `crate::individual` plus the evidence manifest, its token, the
//! time-stamping issuer CRL and the acceptance receipt, in the order the
//! server writes them.

use evidence_verify::certificate::sha256;
use evidence_verify::manifest::{self, role_entry, Role};
use evidence_verify::receipt::{self, Receipt};
use evidence_verify::report::{ComponentKind, InstantSource, Profile};
use evidence_verify::statements::{decode_consent, uuid_text};
use evidence_verify::{validate, Outcome};

use crate::individual::{instants, with_instants};
use crate::support::{assert_complete, component, fixture, policy, Package, DOCUMENT};

/// A complete v2 export accepted at the instants of
/// [`crate::individual::instants`].
pub fn complete() -> Package {
    let (registered, accepted) = instants();
    v2_from(with_instants(registered, accepted), accepted)
}

/// Add the manifest entries to the v1 package `package` and a receipt for
/// acceptance at `accepted`, keeping `INSTRUCCIONES.md` last.
pub fn v2_from(mut package: Package, accepted: i64) -> Package {
    let instructions = package.get("INSTRUCCIONES.md").to_vec();
    package
        .remove("INSTRUCCIONES.md")
        .set("manifiesto.json", fixture!("manifiesto.json"))
        .set("manifiesto.json.tsr", fixture!("manifiesto.json.tsr"))
        .set("tsa-crl.pem", fixture!("root.crl.pem"))
        .set("recibo-aceptacion.json", b"{}")
        .set("INSTRUCCIONES.md", &instructions);
    let receipt = receipt_of(&package, accepted);
    set_receipt(&mut package, &receipt);
    package
}

/// The receipt the server would export for `package` accepted at
/// `accepted`.
pub fn receipt_of(package: &Package, accepted: i64) -> Receipt {
    let consent = decode_consent(package.get("document-consent.bin")).expect("fixture consent");
    Receipt {
        operation_id: uuid_text(consent.operation),
        manifest_sha256: sha256(package.get("manifiesto.json")),
        accepted_at_seconds: accepted,
        accepted_at_nanoseconds: 0,
    }
}

pub fn set_receipt(package: &mut Package, receipt: &Receipt) {
    package.set(
        "recibo-aceptacion.json",
        &receipt::encode(receipt).expect("receipt encodes"),
    );
}

/// Replace the manifest with the result of `edit` over the fixture one and
/// write a receipt over the new bytes, so that only the edited rule breaks
/// besides the manifest token imprint.
pub fn edit_manifest(package: &mut Package, edit: impl FnOnce(&mut manifest::Manifest)) {
    let mut parsed = manifest::parse(fixture!("manifiesto.json")).expect("fixture manifest");
    edit(&mut parsed);
    let bytes = manifest::encode(&parsed).expect("edited manifest encodes");
    package.set("manifiesto.json", &bytes);
    let accepted = receipt::parse(package.get("recibo-aceptacion.json"))
        .expect("receipt")
        .accepted_at_seconds;
    let receipt = receipt_of(package, accepted);
    set_receipt(package, &receipt);
}

#[test]
fn the_fixture_manifest_covers_the_entries_of_the_test_package() {
    let package = complete();
    let parsed = manifest::parse(package.get("manifiesto.json")).expect("fixture manifest");
    assert_eq!(parsed.artifacts.len(), Role::ALL.len());
    for artifact in &parsed.artifacts {
        assert_eq!(artifact.entry, role_entry(artifact.role, DOCUMENT));
        let bytes = package.get(&artifact.entry);
        assert_eq!(artifact.sha256, sha256(bytes), "{}", artifact.entry);
        assert_eq!(artifact.size, bytes.len() as u64, "{}", artifact.entry);
    }
    let (registered, _) = instants();
    assert_eq!(parsed.signers[0].binding_registered_at, registered);
}

#[test]
fn complete_individual_signing_v2_is_checked() {
    let report = validate(&complete().zip(), &policy());
    assert_eq!(report.profile, Some(Profile::IndividualSigningV2));
    assert_complete(&report);
    assert_eq!(
        report.components.iter().map(|c| c.kind).collect::<Vec<_>>(),
        ComponentKind::ALL.to_vec()
    );
    for kind in ComponentKind::ALL {
        assert_eq!(
            component(&report, kind).outcome,
            Outcome::Checked,
            "{kind:?}"
        );
    }
    let manifest = component(&report, ComponentKind::Manifest);
    assert!(manifest
        .findings
        .iter()
        .any(|finding| finding.code == "token.verified"
            && finding.detail.starts_with(manifest::TOKEN_ENTRY)));
}

#[test]
fn the_manifest_token_and_the_receipt_add_their_instants() {
    let (_, accepted) = instants();
    let report = validate(&complete().zip(), &policy());
    let labelled = |label: &str| {
        report
            .instants
            .iter()
            .find(|instant| instant.label == label)
            .unwrap_or_else(|| panic!("no {label} instant: {report:#?}"))
            .clone()
    };
    let token = labelled("manifest token genTime");
    assert_eq!(token.source, InstantSource::Token);
    let document = labelled("document token genTime");
    assert!(document.unix <= token.unix);
    let receipt = labelled("receipt accepted_at");
    assert_eq!(receipt.source, InstantSource::ApplicationRecord);
    assert_eq!(receipt.unix, accepted);
}

#[test]
fn the_shared_time_stamping_certificate_appears_once_in_the_current_status() {
    let report = validate(&complete().zip(), &policy());
    let tsa = report
        .current_status
        .iter()
        .filter(|finding| finding.code == "current.within_validity")
        .filter(|finding| finding.detail == "time-stamping certificate")
        .count();
    assert_eq!(tsa, 1, "{report:#?}");
}
