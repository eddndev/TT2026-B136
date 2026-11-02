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
    // Profiles with a fixed document admit no free name, so their names
    // never collide with an uploaded document and are not reserved.
    let mut from_specs: Vec<&str> = super::SPECS
        .iter()
        .filter(|spec| spec.document.is_none())
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

const CONTRACT_PARTY_1: [&str; 10] = [
    "parte-1-declaracion.bin",
    "parte-1-declaracion.sig",
    "parte-1-certificado.pem",
    "parte-1-crl.pem",
    "parte-1-vinculo.bin",
    "parte-1-vinculo.sig",
    "parte-1-vinculo-raiz.der",
    "parte-1-vinculo-crl.der",
    "parte-1-declaracion.sig.tsr",
    "parte-1-tsa-chain.pem",
];

const CONTRACT_PARTY_2: [&str; 8] = [
    "parte-2-declaracion.bin",
    "parte-2-declaracion.sig",
    "parte-2-certificado.pem",
    "parte-2-crl.pem",
    "parte-2-vinculo.bin",
    "parte-2-vinculo.sig",
    "parte-2-vinculo-raiz.der",
    "parte-2-vinculo-crl.der",
];

fn contract_full() -> Vec<&'static str> {
    let mut names = vec![
        "contrato.pdf",
        "contrato.pdf.tsr",
        "contrato-solicitud.bin",
        "contrato-manifiesto.json",
        "contrato-manifiesto.json.tsr",
        "contrato-recibo.json",
        "ca.pem",
        "tsa-chain.pem",
        "INSTRUCCIONES.md",
    ];
    names.extend(CONTRACT_PARTY_1);
    names.extend(CONTRACT_PARTY_2);
    names
}

fn contract_partial() -> Vec<&'static str> {
    let mut names = vec![
        "contrato.pdf",
        "contrato-solicitud.bin",
        "contrato-acuse-parcial.json",
        "ca.pem",
    ];
    names.extend(CONTRACT_PARTY_1);
    names
}

#[test]
fn recognizes_both_contract_profiles_with_their_fixed_document() {
    let full = contract_full();
    assert_eq!(full.len(), 27);
    assert_eq!(
        recognize(&full).expect("full"),
        Layout {
            profile: Profile::BilateralContractV1,
            document: "contrato.pdf".to_owned()
        }
    );
    let partial = contract_partial();
    assert_eq!(partial.len(), 14);
    assert_eq!(
        recognize(&partial).expect("partial").profile,
        Profile::BilateralContractPartialV1
    );
    let mut instructed = partial.clone();
    instructed.push("INSTRUCCIONES.md");
    assert_eq!(
        recognize(&instructed).expect("partial").profile,
        Profile::BilateralContractPartialV1
    );
}

#[test]
fn removing_any_mandatory_entry_of_a_full_package_is_a_missing_entry() {
    let full = contract_full();
    for removed in full.iter().filter(|name| **name != "INSTRUCCIONES.md") {
        let names: Vec<&str> = full.iter().copied().filter(|n| n != removed).collect();
        let finding = recognize(&names).expect_err(removed);
        assert_eq!(
            (finding.outcome, finding.code, finding.detail.as_str()),
            (Outcome::NotSupported, "profile.missing_entry", *removed),
        );
    }
}

#[test]
fn contract_profiles_admit_no_free_or_foreign_entry() {
    let mut extra = contract_full();
    extra.push("contrato-acuse-parcial.json");
    assert_eq!(refused(&extra), "profile.unknown_entry");
    let mut document = contract_partial();
    document.extend(["otro.pdf", "otro.pdf.sig", "otro.pdf.tsr"]);
    assert_eq!(refused(&document), "profile.unknown_entry");
    let mut sealed = contract_partial();
    sealed.push("contrato.pdf.sig");
    assert_eq!(refused(&sealed), "profile.unknown_entry");
    let mut renamed = contract_partial();
    renamed.retain(|name| *name != "contrato-acuse-parcial.json");
    renamed.extend(["contrato-manifiesto.json", "contrato-recibo.json"]);
    assert_eq!(refused(&renamed), "profile.missing_entry");
}

#[test]
fn no_contract_entry_is_a_reserved_document_name() {
    for name in contract_full().iter().chain(&contract_partial()) {
        let fixed_elsewhere = ["ca.pem", "tsa-chain.pem", "INSTRUCCIONES.md"];
        if !fixed_elsewhere.contains(name) {
            assert!(!FIXED_NAMES.contains(name), "{name}");
        }
    }
    assert!(!is_reserved_document_name("contrato.pdf"));
}
