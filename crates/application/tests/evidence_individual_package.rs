#[path = "evidence_individual_cases/manifest.rs"]
mod manifest;
#[allow(dead_code)]
#[path = "support/individual_evidence.rs"]
mod support;

use std::sync::Mutex;

use application::{
    documents::IndividualSigningEvidence,
    evidence::{export_individual_with_ports, export_with_ports, EvidenceRequest},
};
use domain::{
    crypto::{ArchiveEntry, ArchiveWriter, DocumentHasher},
    DomainError,
};

use support::{fixture, Hasher};

#[derive(Default)]
struct Archive(Mutex<Vec<ArchiveEntry>>);
impl ArchiveWriter for Archive {
    fn write_archive(&self, entries: &[ArchiveEntry]) -> Result<Vec<u8>, DomainError> {
        *self.0.lock().unwrap() = entries.to_vec();
        let mut bytes = Vec::new();
        for entry in entries {
            bytes.extend_from_slice(entry.name().as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(&(entry.content().len() as u64).to_be_bytes());
            bytes.extend_from_slice(entry.content());
        }
        Ok(bytes)
    }
}

fn request<'a>(sealed: &'a application::documents::SealedEvidence) -> EvidenceRequest<'a> {
    EvidenceRequest {
        document_name: "contract.pdf",
        document: b"PDF fixture",
        signature: &sealed.signature,
        timestamp_token: &sealed.timestamp_token,
        signer_certificate_pem: &sealed.signer_certificate_pem,
        issuer_certificate_pem: &sealed.issuer_certificate_pem,
        crl_pem: &sealed.crl_pem,
        tsa_chain_pem: sealed.tsa_chain_pem.as_deref(),
        openssl_version: &sealed.openssl_version,
    }
}

fn contents<'a>(entries: &'a [ArchiveEntry], name: &str) -> &'a [u8] {
    entries
        .iter()
        .find(|entry| entry.name() == name)
        .unwrap()
        .content()
}

#[test]
fn individual_archive_retains_existing_evidence_and_adds_exact_public_proofs() {
    let (record, mut sealed, individual) = fixture();
    sealed.tsa_chain_pem = Some(b"captured-tsa-chain".to_vec());
    let request = request(&sealed);
    let legacy_writer = Archive::default();
    let legacy = export_with_ports(&Hasher, &legacy_writer, &request).unwrap();
    let writer = Archive::default();
    let exported =
        export_individual_with_ports(&Hasher, &writer, &request, &individual, None).unwrap();
    assert_eq!(exported.document_digest_hex, record.digest.to_hex());
    assert_eq!(exported.document_digest_hex, legacy.document_digest_hex);
    let entries = writer.0.lock().unwrap();
    let old_entries = legacy_writer.0.lock().unwrap();
    for entry in old_entries
        .iter()
        .filter(|entry| entry.name() != "INSTRUCCIONES.md")
    {
        assert_eq!(contents(&entries, entry.name()), entry.content());
    }
    assert_eq!(entries.len(), old_entries.len() + 7);
    for (name, expected) in [
        (
            "document-consent.bin",
            individual.consent_statement.as_slice(),
        ),
        (
            "document-consent.sig",
            individual.consent_signature.as_slice(),
        ),
        (
            "certificate-binding.bin",
            individual.binding_statement.as_slice(),
        ),
        (
            "certificate-binding.sig",
            individual.binding_signature.as_slice(),
        ),
        ("binding-root.der", individual.binding_root_der.as_slice()),
        ("binding-crl.der", individual.binding_crl_der.as_slice()),
    ] {
        assert_eq!(contents(&entries, name), expected);
    }
    let decoded: IndividualSigningEvidence =
        serde_json::from_slice(contents(&entries, "individual-signing.json")).unwrap();
    assert_eq!(decoded, individual);
    assert_eq!(
        contents(&entries, "INSTRUCCIONES.md"),
        exported.instructions.as_bytes()
    );
    assert_eq!(
        export_with_ports(&Hasher, &Archive::default(), &request).unwrap(),
        legacy,
        "adding individual export must preserve the existing export result"
    );
}

#[test]
fn instructions_verify_each_proof_and_explain_historical_time_and_internal_trust() {
    let (_, sealed, individual) = fixture();
    let exported = export_individual_with_ports(
        &Hasher,
        &Archive::default(),
        &request(&sealed),
        &individual,
        None,
    )
    .unwrap();
    let text = &exported.instructions;
    for expected in [
        "openssl dgst -sha256 -verify firmante.pub.pem -signature certificate-binding.sig certificate-binding.bin",
        "openssl dgst -sha256 -verify firmante.pub.pem -signature document-consent.sig document-consent.bin",
        "openssl verify -attime 100 -crl_check",
        "openssl verify -attime 300 -crl_check",
        "openssl x509 -inform DER -in binding-root.der",
        "openssl crl -inform DER -in binding-crl.der",
        "no demuestra que la firma personal ya existiera",
        "no constituye una constancia de un PSC autorizado",
        "canal independiente",
        "DOCBIND1",
        "DOCSIGN1",
        "subject",
        "cuenta",
    ] {
        assert!(text.contains(expected), "instructions omit {expected}");
    }
    for bytes in [
        individual.binding_statement.as_slice(),
        individual.consent_statement.as_slice(),
        sealed.signature.as_slice(),
    ] {
        assert!(text.contains(&Hasher.hash_bytes(bytes).to_hex()));
    }
    assert!(!text.contains("{{"));
}

#[test]
fn all_colliding_document_signature_and_reserved_names_fail_before_archiving() {
    let (_, sealed, individual) = fixture();
    for name in [
        "document-consent",
        "document-consent.bin",
        "document-consent.sig",
        "certificate-binding",
        "certificate-binding.bin",
        "certificate-binding.sig",
        "binding-root.der",
        "binding-crl.der",
        "individual-signing.json",
        "certificado.pem",
        "ca.pem",
        "crl.pem",
        "INSTRUCCIONES.md",
    ] {
        let writer = Archive::default();
        let request = EvidenceRequest {
            document_name: name,
            ..request(&sealed)
        };
        assert!(
            export_individual_with_ports(&Hasher, &writer, &request, &individual, None).is_err(),
            "duplicate archive name was admitted: {name}"
        );
        assert!(writer.0.lock().unwrap().is_empty());
    }
}

/// A signature accepted before the evidence manifest existed may carry a
/// name that upload now reserves; its package without a manifest has no
/// duplicate entry and stays exportable.
#[test]
fn a_package_without_a_manifest_admits_names_reserved_after_its_acceptance() {
    let (_, sealed, individual) = fixture();
    for name in [
        "CA.PEM",
        "Crl.PEM",
        "individual-signing.JSON",
        "manifiesto.json",
        "Manifiesto.JSON",
        "tsa-crl.pem",
        "recibo-aceptacion.json",
        "TSA-CHAIN.pem",
    ] {
        let request = EvidenceRequest {
            document_name: name,
            ..request(&sealed)
        };
        assert!(
            export_individual_with_ports(&Hasher, &Archive::default(), &request, &individual, None)
                .is_ok(),
            "a historical package was refused: {name}"
        );
    }
}

#[test]
fn malformed_individual_format_or_statement_cannot_be_packaged_as_supported_evidence() {
    let (_, sealed, original) = fixture();
    for field in 0..4 {
        let mut individual = original.clone();
        match field {
            0 => individual.format_version = 2,
            1 => individual.binding_statement.push(0),
            2 => individual.consent_statement[0] ^= 1,
            _ => individual.consent_signature.clear(),
        }
        let writer = Archive::default();
        assert!(export_individual_with_ports(
            &Hasher,
            &writer,
            &request(&sealed),
            &individual,
            None
        )
        .is_err());
        assert!(writer.0.lock().unwrap().is_empty());
    }
}
