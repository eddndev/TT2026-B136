use super::{is_reserved_document_name, recognize, Layout, FIXED_NAMES};
use crate::report::{Outcome, Profile};

const SEAL: [&str; 8] = [
    "acta.pdf",
    "acta.pdf.sig",
    "acta.pdf.tsr",
    "certificado.pem",
    "ca.pem",
    "crl.pem",
    "tsa-chain.pem",
    "INSTRUCCIONES.md",
];

const INDIVIDUAL_EXTRA: [&str; 7] = [
    "document-consent.bin",
    "document-consent.sig",
    "certificate-binding.bin",
    "certificate-binding.sig",
    "binding-root.der",
    "binding-crl.der",
    "individual-signing.json",
];

const MANIFEST_EXTRA: [&str; 4] = [
    "manifiesto.json",
    "manifiesto.json.tsr",
    "tsa-crl.pem",
    "recibo-aceptacion.json",
];

fn refused(names: &[&str]) -> &'static str {
    let finding = recognize(names).expect_err("not a profile");
    assert_eq!(finding.outcome, Outcome::NotSupported);
    finding.code
}

#[test]
fn recognizes_each_profile_by_its_inventory() {
    let seal = recognize(&SEAL).expect("server seal");
    assert_eq!(
        seal,
        Layout {
            profile: Profile::ServerSealV1,
            document: "acta.pdf".to_owned()
        }
    );
    assert_eq!(seal.signature(), "acta.pdf.sig");
    assert_eq!(seal.token(), "acta.pdf.tsr");
    let mut individual: Vec<&str> = SEAL.to_vec();
    individual.extend(INDIVIDUAL_EXTRA);
    assert_eq!(
        recognize(&individual).expect("v1").profile,
        Profile::IndividualSigningV1
    );
    individual.extend(MANIFEST_EXTRA);
    assert_eq!(
        recognize(&individual).expect("v2").profile,
        Profile::IndividualSigningV2
    );
}

#[test]
fn order_does_not_matter_and_optional_entries_may_be_absent() {
    let mut names: Vec<&str> = SEAL
        .iter()
        .copied()
        .filter(|name| *name != "tsa-chain.pem" && *name != "INSTRUCCIONES.md")
        .collect();
    names.reverse();
    assert_eq!(
        recognize(&names).expect("seal").profile,
        Profile::ServerSealV1
    );
}

#[test]
fn the_tsa_chain_is_required_for_individual_signing() {
    let mut names: Vec<&str> = SEAL
        .iter()
        .copied()
        .filter(|name| *name != "tsa-chain.pem")
        .collect();
    names.extend(INDIVIDUAL_EXTRA);
    assert_eq!(refused(&names), "profile.missing_entry");
}

#[test]
fn a_seal_document_may_take_a_name_fixed_by_individual_signing() {
    let names = [
        "individual-signing.json",
        "individual-signing.json.sig",
        "individual-signing.json.tsr",
        "certificado.pem",
        "ca.pem",
        "crl.pem",
    ];
    let layout = recognize(&names).expect("seal");
    assert_eq!(layout.document, "individual-signing.json");
}

#[test]
fn rejects_unknown_ambiguous_and_missing_inventories() {
    let mut unknown = SEAL.to_vec();
    unknown.push("extra.txt");
    assert_eq!(refused(&unknown), "profile.unknown_entry");
    let mut ambiguous = SEAL.to_vec();
    ambiguous.extend(["b.pdf", "b.pdf.sig", "b.pdf.tsr"]);
    assert_eq!(refused(&ambiguous), "profile.ambiguous_document");
    assert_eq!(refused(&SEAL[1..]), "profile.no_document");
    assert_eq!(refused(&SEAL[..5]), "profile.missing_entry");
    let mut partial = SEAL.to_vec();
    partial.extend(&INDIVIDUAL_EXTRA[..6]);
    let finding = recognize(&partial).expect_err("missing json");
    assert_eq!(finding.code, "profile.missing_entry");
    assert_eq!(finding.detail, "individual-signing.json");
}

#[test]
fn names_differing_only_in_case_are_different() {
    let names: Vec<&str> = SEAL
        .iter()
        .map(|name| if *name == "crl.pem" { "CRL.pem" } else { name })
        .collect();
    assert_eq!(refused(&names), "profile.missing_entry");
}

#[test]
fn fixed_names_are_the_union_of_every_profile_and_the_instructions() {
    let mut expected: Vec<&str> = SEAL[3..].to_vec();
    expected.extend(INDIVIDUAL_EXTRA);
    expected.extend(MANIFEST_EXTRA);
    expected.sort_unstable();
    let mut fixed = FIXED_NAMES.to_vec();
    fixed.sort_unstable();
    assert_eq!(fixed, expected);
    let mut from_specs: Vec<&str> = super::SPECS
        .iter()
        .flat_map(|spec| spec.required.iter().chain(spec.optional))
        .copied()
        .chain([super::INSTRUCTIONS])
        .collect();
    from_specs.sort_unstable();
    from_specs.dedup();
    assert_eq!(from_specs, fixed);
}

#[test]
fn a_document_name_is_reserved_when_it_or_a_sibling_is_fixed_ignoring_case() {
    for reserved in [
        "ca.pem",
        "CA.PEM",
        "Manifiesto.JSON",
        "manifiesto.json",
        "MANIFIESTO.JSON",
        "certificado.pem",
        "instrucciones.md",
        "Recibo-Aceptacion.json",
        "TSA-CRL.pem",
    ] {
        assert!(is_reserved_document_name(reserved), "{reserved}");
    }
    for free in [
        "acta.pdf",
        "ACTA.PDF",
        "manifiesto.pdf",
        "ca",
        "crl.pem.txt",
        "",
    ] {
        assert!(!is_reserved_document_name(free), "{free}");
    }
}
