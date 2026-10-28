use super::super::admit;
use super::assert_not_supported;
use super::writer::{central, local, write};

const KIB: usize = 1024;
const MIB: usize = 1024 * KIB;

/// Names fixed only in the individual signing profiles, limited to 64 KiB
/// unless they are a document candidate.
const SMALL_INDIVIDUAL_NAMES: [&str; 8] = [
    "binding-root.der",
    "certificate-binding.bin",
    "certificate-binding.sig",
    "document-consent.bin",
    "document-consent.sig",
    "manifiesto.json",
    "manifiesto.json.tsr",
    "recibo-aceptacion.json",
];

/// Names fixed in every profile; they can never be the document.
const COMMON_SMALL_NAMES: [&str; 4] = [
    "certificado.pem",
    "ca.pem",
    "tsa-chain.pem",
    "INSTRUCCIONES.md",
];

const CRL_NAMES: [&str; 3] = ["crl.pem", "binding-crl.der", "tsa-crl.pem"];

/// Entries with no fixed name that are not a document candidate.
const OTHER_NAMES: [&str; 5] = [
    "acta.pdf",
    "acta.pdf.sig",
    "acta.pdf.tsr",
    "CA.pem",
    "crl.pem.bak",
];

/// `entries` with entry `index` declaring `size` bytes in both headers
/// without carrying them, so a limit must refuse it before any layout check.
fn declaring(entries: &[(&str, &[u8])], index: usize, size: usize) -> Vec<u8> {
    let mut built = write(entries);
    for at in [local::SIZE, local::COMPRESSED_SIZE] {
        built.set_u32(built.locals[index] + at, size as u32);
    }
    for at in [central::SIZE, central::COMPRESSED_SIZE] {
        built.set_u32(built.centrals[index] + at, size as u32);
    }
    built.bytes
}

/// A package whose entry `name` holds `size` bytes, followed by `siblings`.
fn package(name: &str, size: usize, siblings: &[&str]) -> Vec<u8> {
    let content = vec![0x5a; size];
    let mut entries: Vec<(&str, &[u8])> = vec![(name, &content)];
    entries.extend(siblings.iter().map(|sibling| (*sibling, &b"s"[..])));
    write(&entries).bytes
}

/// `name` is admitted at `limit` bytes and refused one byte later.
fn limited(name: &str, limit: usize, siblings: &[&str]) {
    let archive = admit(&package(name, limit, siblings)).unwrap();
    assert_eq!(archive.get(name).map(<[u8]>::len), Some(limit), "{name}");
    assert_not_supported(&package(name, limit + 1, siblings), "zip.entry_too_large");
}

fn candidate_siblings(name: &str) -> [String; 2] {
    [format!("{name}.sig"), format!("{name}.tsr")]
}

#[test]
fn thirty_two_entries_are_admitted_and_thirty_three_are_not() {
    let names: Vec<String> = (0..33).map(|index| format!("e{index}.bin")).collect();
    let entries: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"x"[..])).collect();
    let archive = admit(&write(&entries[..32]).bytes).unwrap();
    assert_eq!(archive.entries.len(), 32);
    assert_not_supported(&write(&entries).bytes, "zip.too_many_entries");
}

#[test]
fn small_fixed_entries_are_limited_to_64_kib() {
    for name in SMALL_INDIVIDUAL_NAMES.iter().chain(&COMMON_SMALL_NAMES) {
        limited(name, 64 * KIB, &[]);
    }
}

#[test]
fn crl_entries_are_limited_to_1_mib() {
    for name in CRL_NAMES {
        limited(name, MIB, &[]);
    }
}

#[test]
fn the_individual_signing_statement_is_limited_to_4_5_mib() {
    limited("individual-signing.json", 4_718_592, &[]);
}

#[test]
fn entries_without_a_fixed_name_or_both_siblings_are_limited_to_64_kib() {
    for name in OTHER_NAMES {
        limited(name, 64 * KIB, &[]);
    }
    limited("acta.pdf", 64 * KIB, &["acta.pdf.sig"]);
    limited("acta.pdf", 64 * KIB, &["acta.pdf.tsr"]);
}

#[test]
fn the_signature_and_response_of_a_document_are_limited_to_64_kib() {
    let content = vec![0x5a; 64 * KIB + 1];
    for index in [1, 2] {
        let mut entries: [(&str, &[u8]); 3] = [
            ("acta.pdf", b"doc"),
            ("acta.pdf.sig", b"s"),
            ("acta.pdf.tsr", b"t"),
        ];
        entries[index].1 = &content;
        assert_not_supported(&write(&entries).bytes, "zip.entry_too_large");
    }
}

#[test]
fn a_document_candidate_is_limited_to_16_mib() {
    let [signature, token] = candidate_siblings("acta.pdf");
    let siblings = [signature.as_str(), token.as_str()];
    let archive = admit(&package("acta.pdf", 16 * MIB, &siblings)).unwrap();
    assert_eq!(archive.get("acta.pdf").map(<[u8]>::len), Some(16 * MIB));
    let entries: [(&str, &[u8]); 3] = [("acta.pdf", b""), (&signature, b"s"), (&token, b"t")];
    assert_not_supported(&declaring(&entries, 0, 16 * MIB + 1), "zip.entry_too_large");
}

#[test]
fn a_server_seal_document_may_carry_an_individual_entry_name() {
    for (name, size) in [
        ("manifiesto.json", 100 * KIB),
        ("binding-crl.der", 2 * MIB),
        ("individual-signing.json", 5 * MIB),
    ] {
        let [signature, token] = candidate_siblings(name);
        let bytes = package(name, size, &[&signature, &token, "certificado.pem"]);
        let archive = admit(&bytes).unwrap_or_else(|finding| panic!("{name}: {finding:?}"));
        assert_eq!(archive.get(name).map(<[u8]>::len), Some(size), "{name}");
    }
}

#[test]
fn names_fixed_in_every_profile_keep_their_limit_with_siblings() {
    for name in COMMON_SMALL_NAMES {
        let [signature, token] = candidate_siblings(name);
        limited(name, 64 * KIB, &[&signature, &token]);
    }
    let [signature, token] = candidate_siblings("crl.pem");
    limited("crl.pem", MIB, &[&signature, &token]);
}

#[test]
fn the_total_is_limited_to_25_mib() {
    let document = vec![0u8; 16 * MIB];
    let rest = vec![1u8; 9 * MIB];
    let entries: [(&str, &[u8]); 6] = [
        ("a.bin", &document),
        ("b.bin", &rest),
        ("a.bin.sig", b""),
        ("a.bin.tsr", b""),
        ("b.bin.sig", b""),
        ("b.bin.tsr", b""),
    ];
    let archive = admit(&write(&entries).bytes).unwrap();
    assert_eq!(archive.entries.len(), 6);

    let mut over = entries;
    over[1].1 = b"";
    assert_not_supported(&declaring(&over, 1, 9 * MIB + 1), "zip.total_too_large");
}
