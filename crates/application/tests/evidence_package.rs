//! Behavior of the evidence package export use case with the hashing
//! and archive ports mocked: entry set, entry order, instruction
//! placeholder filling, and error propagation.

use std::io::Read;
use std::sync::{Arc, Mutex};

use application::evidence::{EvidenceRequest, ExportEvidencePackage};
use application::ApplicationError;
use domain::crypto::archive::{ArchiveEntry, ArchiveWriter};
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::DomainError;
use mockall::mock;

mock! {
    Hasher {}

    impl DocumentHasher for Hasher {
        fn hash_bytes(&self, data: &[u8]) -> Sha256Digest;
        fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError>;
    }
}

mock! {
    Archiver {}

    impl ArchiveWriter for Archiver {
        fn write_archive(&self, entries: &[ArchiveEntry]) -> Result<Vec<u8>, DomainError>;
    }
}

/// Entries the mock archiver observed: name and content pairs.
type SeenEntries = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

fn digest() -> Sha256Digest {
    Sha256Digest::from_array([0xab; 32])
}

fn document_hasher() -> MockHasher {
    let mut hasher = MockHasher::new();
    hasher
        .expect_hash_bytes()
        .withf(|data| data == b"cuerpo del documento")
        .times(1)
        .returning(|_| digest());
    hasher
}

fn request<'a>(tsa_chain_pem: Option<&'a [u8]>) -> EvidenceRequest<'a> {
    EvidenceRequest {
        document_name: "acta.txt",
        document: b"cuerpo del documento",
        signature: b"raw signature bytes",
        timestamp_token: b"der token bytes",
        signer_certificate_pem: b"signer pem",
        issuer_certificate_pem: b"issuer pem",
        crl_pem: b"crl pem",
        tsa_chain_pem,
        openssl_version: "OpenSSL 3.5.7 9 Jun 2026",
    }
}

/// An archiver that records the entries it received and returns fixed
/// archive bytes.
fn recording_archiver(seen: SeenEntries) -> MockArchiver {
    let mut archiver = MockArchiver::new();
    archiver
        .expect_write_archive()
        .times(1)
        .returning(move |entries| {
            let mut log = seen.lock().unwrap();
            *log = entries
                .iter()
                .map(|entry| (entry.name().to_string(), entry.content().to_vec()))
                .collect();
            Ok(b"zip bytes".to_vec())
        });
    archiver
}

#[test]
fn the_package_holds_every_artifact_in_order_under_the_documented_names() {
    let seen: SeenEntries = Arc::new(Mutex::new(Vec::new()));
    let use_case = ExportEvidencePackage::new(document_hasher(), recording_archiver(seen.clone()));

    let package = use_case.execute(&request(Some(b"tsa chain pem"))).unwrap();
    assert_eq!(package.archive, b"zip bytes");
    assert_eq!(package.document_digest_hex, digest().to_hex());

    let entries = seen.lock().unwrap();
    let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "acta.txt",
            "acta.txt.sig",
            "acta.txt.tsr",
            "certificado.pem",
            "ca.pem",
            "crl.pem",
            "tsa-chain.pem",
            "INSTRUCCIONES.md",
        ]
    );
    assert_eq!(entries[0].1, b"cuerpo del documento");
    assert_eq!(entries[1].1, b"raw signature bytes");
    assert_eq!(entries[2].1, b"der token bytes");
    assert_eq!(entries[3].1, b"signer pem");
    assert_eq!(entries[4].1, b"issuer pem");
    assert_eq!(entries[5].1, b"crl pem");
    assert_eq!(entries[6].1, b"tsa chain pem");
    assert_eq!(entries[7].1, package.instructions.as_bytes());
}

#[test]
fn without_a_tsa_chain_the_entry_is_absent_but_nothing_else_changes() {
    let seen: SeenEntries = Arc::new(Mutex::new(Vec::new()));
    let use_case = ExportEvidencePackage::new(document_hasher(), recording_archiver(seen.clone()));

    use_case.execute(&request(None)).unwrap();

    let entries = seen.lock().unwrap();
    let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
    assert!(
        !names.contains(&"tsa-chain.pem"),
        "no chain entry may appear when none was supplied: {names:?}"
    );
    assert_eq!(names.len(), 7);
    assert_eq!(names.last(), Some(&"INSTRUCCIONES.md"));
}

#[test]
fn the_instructions_fill_every_placeholder() {
    let seen: SeenEntries = Arc::new(Mutex::new(Vec::new()));
    let use_case = ExportEvidencePackage::new(document_hasher(), recording_archiver(seen));

    let package = use_case.execute(&request(None)).unwrap();
    let text = &package.instructions;

    assert!(
        !text.contains("{{"),
        "no placeholder may survive the fill: {text}"
    );
    for needle in [
        "acta.txt",
        "acta.txt.sig",
        "acta.txt.tsr",
        &digest().to_hex(),
        "OpenSSL 3.5.7 9 Jun 2026",
        "openssl dgst -sha256 -verify",
        "openssl verify -crl_check",
        "openssl ts -verify",
    ] {
        assert!(
            text.contains(needle),
            "the instructions must mention {needle:?}"
        );
    }
}

#[test]
fn with_a_tsa_chain_the_token_check_anchors_on_the_bundled_chain() {
    let seen: SeenEntries = Arc::new(Mutex::new(Vec::new()));
    let use_case = ExportEvidencePackage::new(document_hasher(), recording_archiver(seen));

    let package = use_case.execute(&request(Some(b"tsa chain pem"))).unwrap();
    let text = &package.instructions;

    assert!(
        text.contains("openssl ts -verify -data acta.txt -in acta.txt.tsr -CAfile tsa-chain.pem"),
        "the token check must anchor on the bundled tsa chain: {text}"
    );
    assert!(
        !text.contains("openssl ts -verify -data acta.txt -in acta.txt.tsr -CAfile ca.pem"),
        "the token check must not anchor on the issuer root when a chain travels: {text}"
    );
    // The other documented commands keep their fixed anchors.
    assert!(
        text.contains("openssl verify -crl_check -CAfile ca-y-crl.pem certificado.pem"),
        "the certificate check keeps the issuer root as anchor: {text}"
    );
}

#[test]
fn without_a_tsa_chain_the_token_check_anchors_on_the_issuer_root() {
    let seen: SeenEntries = Arc::new(Mutex::new(Vec::new()));
    let use_case = ExportEvidencePackage::new(document_hasher(), recording_archiver(seen));

    let package = use_case.execute(&request(None)).unwrap();
    let text = &package.instructions;

    assert!(
        text.contains("openssl ts -verify -data acta.txt -in acta.txt.tsr -CAfile ca.pem"),
        "the token check must anchor on the issuer root: {text}"
    );
    assert!(
        !text.contains("-CAfile tsa-chain.pem"),
        "no command may reference an absent chain file: {text}"
    );
}

#[test]
fn a_document_name_unusable_as_an_entry_name_is_rejected() {
    let mut hasher = MockHasher::new();
    hasher.expect_hash_bytes().times(1).returning(|_| digest());
    let mut archiver = MockArchiver::new();
    archiver.expect_write_archive().times(0);

    let use_case = ExportEvidencePackage::new(hasher, archiver);
    let bad_request = EvidenceRequest {
        document_name: "../escape.txt",
        ..request(None)
    };
    let err = use_case.execute(&bad_request).unwrap_err();
    assert!(matches!(
        err,
        ApplicationError::Domain(DomainError::InvalidArchiveEntryName { .. })
    ));
}

#[test]
fn an_archiver_failure_propagates() {
    let mut archiver = MockArchiver::new();
    archiver
        .expect_write_archive()
        .times(1)
        .returning(|_| Err(DomainError::ArchiveWriteFailure("disk full".to_string())));

    let use_case = ExportEvidencePackage::new(document_hasher(), archiver);
    let err = use_case.execute(&request(None)).unwrap_err();
    assert!(matches!(
        err,
        ApplicationError::Domain(DomainError::ArchiveWriteFailure(_))
    ));
}
