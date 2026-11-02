//! Packages of bilateral contract signing built from
//! `tests/contract-fixtures`, whose files carry their package entry names.

use crate::support::{pin, Package};
use evidence_verify::json::{self, Limits, Value};
use evidence_verify::policy::{Policy, Purposes};
use evidence_verify::{certificate, pem, Outcome, Report};

/// The shared entries of a full package, in the order the server writes.
pub const FULL_SHARED: [&str; 8] = [
    "contrato.pdf",
    "contrato.pdf.tsr",
    "contrato-solicitud.bin",
    "contrato-manifiesto.json",
    "contrato-manifiesto.json.tsr",
    "contrato-recibo.json",
    "ca.pem",
    "tsa-chain.pem",
];

/// The position 1 entries, present in both packages.
pub const PARTY_1: [&str; 10] = [
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

/// The position 2 entries of a full package.
pub const PARTY_2: [&str; 8] = [
    "parte-2-declaracion.bin",
    "parte-2-declaracion.sig",
    "parte-2-certificado.pem",
    "parte-2-crl.pem",
    "parte-2-vinculo.bin",
    "parte-2-vinculo.sig",
    "parte-2-vinculo-raiz.der",
    "parte-2-vinculo-crl.der",
];

/// Bytes of one file under `tests/contract-fixtures`.
pub fn contract_fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/contract-fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn package(names: &[&str]) -> Package {
    let mut package = Package {
        entries: Vec::new(),
    };
    for name in names {
        package.set(name, &contract_fixture(name));
    }
    package.set("INSTRUCCIONES.md", b"# Instrucciones\n");
    package
}

/// The full package: 26 mandatory entries and `INSTRUCCIONES.md`.
pub fn full() -> Package {
    let names: Vec<&str> = FULL_SHARED
        .iter()
        .chain(&PARTY_1)
        .chain(&PARTY_2)
        .copied()
        .collect();
    package(&names)
}

/// The partial package: 14 mandatory entries and `INSTRUCCIONES.md`.
pub fn partial() -> Package {
    let mut names = vec![
        "contrato.pdf",
        "contrato-solicitud.bin",
        "contrato-acuse-parcial.json",
        "ca.pem",
    ];
    names.extend(PARTY_1);
    package(&names)
}

/// `package` with every file of `tests/contract-fixtures/variants/{name}`
/// set under its entry name. A variant directory holds only the entries
/// that differ from the base ceremony.
pub fn with_variant(mut package: Package, name: &str) -> Package {
    let directory = format!(
        "{}/tests/contract-fixtures/variants/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let listing =
        std::fs::read_dir(&directory).unwrap_or_else(|error| panic!("{directory}: {error}"));
    let mut names: Vec<String> = listing
        .map(|item| item.expect("variant entry").file_name())
        .map(|file| file.into_string().expect("ASCII entry name"))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "{directory} is empty");
    for entry in names {
        package.set(&entry, &variant_file(name, &entry));
    }
    package
}

/// Bytes of one file of the variant `name`.
pub fn variant_file(name: &str, entry: &str) -> Vec<u8> {
    contract_fixture(&format!("variants/{name}/{entry}"))
}

/// Every distinct failed code of `report`, sorted.
pub fn failed_codes(report: &Report) -> Vec<&'static str> {
    let mut codes: Vec<&'static str> = report
        .components
        .iter()
        .flat_map(|component| component.findings.iter())
        .filter(|finding| finding.outcome == Outcome::Failed)
        .map(|finding| finding.code)
        .collect();
    codes.sort_unstable();
    codes.dedup();
    codes
}

/// One integer of `fixtures.json` at `path`.
pub fn fact(path: &[&str]) -> i64 {
    let bytes = contract_fixture("fixtures.json");
    let value = json::parse(&bytes, Limits { max_values: 10_000 }).expect("fixtures.json");
    let mut at: &Value = &value;
    for key in path {
        at = at.get(key).unwrap_or_else(|| panic!("fact {path:?}"));
    }
    i64::try_from(at.as_integer().expect("integer")).expect("fits")
}

/// A policy that pins the fixture root for every purpose, evaluated one
/// hour after Tf.
pub fn policy() -> Policy {
    let root = pem::decode_one(&contract_fixture("ca.pem"), "CERTIFICATE").expect("root");
    Policy::new(
        vec![pin(
            certificate::sha256(&root),
            Purposes::ALL,
            "contract root",
        )],
        fact(&["instants", "tf"]) + 3600,
    )
}
