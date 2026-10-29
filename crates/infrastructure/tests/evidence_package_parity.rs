//! Packages written by the server export path, from
//! `crates/application/src/evidence.rs` and
//! `crates/application/src/evidence/individual.rs` through
//! `crates/infrastructure/src/archive.rs`, validated by the pure verifier
//! that the browser runs. The material comes from the verifier fixtures in
//! `crates/evidence-verify/tests/fixtures`. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use application::documents::IndividualSigningEvidence;
use application::evidence::{export_individual_with_ports, EvidenceRequest, ExportEvidencePackage};
use evidence_verify::archive;
use evidence_verify::policy::{PinnedRoot, Policy, Purposes};
use evidence_verify::report::{InstantSource, Profile, Report};
use evidence_verify::{validate, Outcome};
use infrastructure::{RingSha256Hasher, StoredZipWriter};
use serde_json::Value;

macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../evidence-verify/tests/fixtures/",
            $name
        ))
        .as_slice()
    };
}

const DOCUMENT: &str = "acta.pdf";
const OPENSSL_VERSION: &str = "OpenSSL 3.5.0 8 Apr 2025";

/// Facts the fixture generator recorded; bytes change on regeneration.
struct Facts {
    gen_time: i64,
    crl_this_update: i64,
    root_sha256: [u8; 32],
    foreign_root_sha256: [u8; 32],
}

fn facts() -> Facts {
    let value: Value = serde_json::from_slice(fixture!("fixtures.json")).expect("fixtures.json");
    let int = |pointer: &str| {
        value
            .pointer(pointer)
            .and_then(Value::as_i64)
            .expect(pointer)
    };
    let digest = |pointer: &str| {
        let text = value
            .pointer(pointer)
            .and_then(Value::as_str)
            .expect(pointer);
        let mut out = [0u8; 32];
        for (index, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).expect("hex");
        }
        out
    };
    Facts {
        gen_time: int("/tokens/token-ess-v1.tsr/gen_time"),
        crl_this_update: int("/crls/root.crl.der/this_update"),
        root_sha256: digest("/certificates/root.der/sha256"),
        foreign_root_sha256: digest("/certificates/foreign-root.der/sha256"),
    }
}

/// A policy pinning `root` for every purpose, one hour after the tokens.
fn policy_pinning(root: [u8; 32]) -> Policy {
    let pinned = PinnedRoot {
        sha256: root,
        purposes: Purposes::ALL,
        label: "fixture root".to_owned(),
    };
    Policy::new(vec![pinned], facts().gen_time + 3600)
}

fn seal_request<'a>(token: &'a [u8], chain: Option<&'a [u8]>) -> EvidenceRequest<'a> {
    EvidenceRequest {
        document_name: DOCUMENT,
        document: fixture!("document.bin"),
        signature: fixture!("server-signer.sig"),
        timestamp_token: token,
        signer_certificate_pem: fixture!("server-signer.pem"),
        issuer_certificate_pem: fixture!("root.pem"),
        crl_pem: fixture!("root.crl.pem"),
        tsa_chain_pem: chain,
        openssl_version: OPENSSL_VERSION,
    }
}

fn server_seal(token: &[u8], chain: Option<&[u8]>) -> Vec<u8> {
    ExportEvidencePackage::new(RingSha256Hasher::new(), StoredZipWriter::new())
        .execute(&seal_request(token, chain))
        .expect("server seal export")
        .archive
}

fn assert_complete(report: &Report, profile: Profile) {
    assert_eq!(report.profile, Some(profile), "{report:#?}");
    assert_eq!(report.overall, Outcome::Checked, "{report:#?}");
    for component in &report.components {
        assert!(
            matches!(component.outcome, Outcome::Checked | Outcome::NotApplicable),
            "{report:#?}"
        );
    }
}

/// The admitted archive holds exactly `expected`, in order and byte for
/// byte, the report lists the same names, and the archive has no byte
/// beyond the stored layout: each local header of 30 bytes plus name and
/// content, each central header of 46 bytes plus name, and the 22-byte end
/// record.
fn assert_admitted_exactly(zip: &[u8], report: &Report, expected: &[(&str, &[u8])]) {
    let admitted = archive::admit(zip).expect("server archive is admitted");
    let names: Vec<&str> = expected.iter().map(|(name, _)| *name).collect();
    assert_eq!(admitted.names(), names);
    for (entry, (name, data)) in admitted.entries.iter().zip(expected) {
        assert_eq!(entry.data.as_slice(), *data, "content of {name}");
    }
    let reported: Vec<&str> = report
        .digests
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(reported, names);
    let layout: usize = expected
        .iter()
        .map(|(name, data)| 30 + 46 + 2 * name.len() + data.len())
        .sum::<usize>()
        + 22;
    assert_eq!(zip.len(), layout);
}

fn instructions(zip: &[u8]) -> Vec<u8> {
    archive::admit(zip)
        .expect("admitted")
        .get("INSTRUCCIONES.md")
        .expect("instructions")
        .to_vec()
}

#[test]
fn a_server_seal_export_is_complete() {
    let chain = Some(fixture!("root.pem"));
    for token in [
        fixture!("token-ess-v1.tsr"),
        fixture!("token-ess-v2.tsr"),
        fixture!("token-bare-v2.der"),
    ] {
        let zip = server_seal(token, chain);
        let report = validate(&zip, &policy_pinning(facts().root_sha256));
        assert_complete(&report, Profile::ServerSealV1);
        let token_instant = report
            .instants
            .iter()
            .find(|instant| instant.source == InstantSource::Token)
            .expect("token instant");
        assert_eq!(token_instant.unix, facts().gen_time);
        let text = instructions(&zip);
        assert_admitted_exactly(
            &zip,
            &report,
            &[
                (DOCUMENT, fixture!("document.bin")),
                ("acta.pdf.sig", fixture!("server-signer.sig")),
                ("acta.pdf.tsr", token),
                ("certificado.pem", fixture!("server-signer.pem")),
                ("ca.pem", fixture!("root.pem")),
                ("crl.pem", fixture!("root.crl.pem")),
                ("tsa-chain.pem", fixture!("root.pem")),
                ("INSTRUCCIONES.md", &text),
            ],
        );
    }
}

#[test]
fn a_server_seal_export_without_a_tsa_chain_is_complete() {
    let token = fixture!("token-ess-v1.tsr");
    let zip = server_seal(token, None);
    let report = validate(&zip, &policy_pinning(facts().root_sha256));
    assert_complete(&report, Profile::ServerSealV1);
    let text = instructions(&zip);
    assert_admitted_exactly(
        &zip,
        &report,
        &[
            (DOCUMENT, fixture!("document.bin")),
            ("acta.pdf.sig", fixture!("server-signer.sig")),
            ("acta.pdf.tsr", token),
            ("certificado.pem", fixture!("server-signer.pem")),
            ("ca.pem", fixture!("root.pem")),
            ("crl.pem", fixture!("root.crl.pem")),
            ("INSTRUCCIONES.md", &text),
        ],
    );
}

#[test]
fn a_server_seal_export_under_an_unpinned_root_is_not_complete() {
    let zip = server_seal(fixture!("token-ess-v1.tsr"), Some(fixture!("root.pem")));
    let report = validate(&zip, &policy_pinning(facts().foreign_root_sha256));
    assert_eq!(report.profile, Some(Profile::ServerSealV1));
    assert_eq!(report.overall, Outcome::NotSupported, "{report:#?}");
}

fn individual_evidence(registered: i64, accepted: i64) -> IndividualSigningEvidence {
    IndividualSigningEvidence {
        format_version: 1,
        binding_statement: fixture!("certificate-binding.bin").to_vec(),
        binding_signature: fixture!("certificate-binding.sig").to_vec(),
        binding_root_der: fixture!("root.der").to_vec(),
        binding_crl_der: fixture!("root.crl.der").to_vec(),
        binding_registered_at: registered,
        consent_statement: fixture!("document-consent.bin").to_vec(),
        consent_signature: fixture!("document-consent.sig").to_vec(),
        accepted_at: accepted,
    }
}

#[test]
fn an_individual_signing_export_is_complete() {
    let facts = facts();
    let (registered, accepted) = (facts.crl_this_update + 60, facts.gen_time + 60);
    let individual = individual_evidence(registered, accepted);
    let token = fixture!("token-ess-v1.tsr");
    let request = EvidenceRequest {
        document_name: DOCUMENT,
        document: fixture!("document.bin"),
        signature: fixture!("holder.sig"),
        timestamp_token: token,
        signer_certificate_pem: fixture!("holder.pem"),
        issuer_certificate_pem: fixture!("root.pem"),
        crl_pem: fixture!("root.crl.pem"),
        tsa_chain_pem: Some(fixture!("root.pem")),
        openssl_version: OPENSSL_VERSION,
    };
    let zip = export_individual_with_ports(
        &RingSha256Hasher::new(),
        &StoredZipWriter::new(),
        &request,
        &individual,
        None,
    )
    .expect("individual export")
    .archive;
    let report = validate(&zip, &policy_pinning(facts.root_sha256));
    assert_complete(&report, Profile::IndividualSigningV1);
    let records: Vec<i64> = report
        .instants
        .iter()
        .filter(|instant| instant.source == InstantSource::ApplicationRecord)
        .map(|instant| instant.unix)
        .collect();
    assert_eq!(records, vec![registered, accepted]);
    let json = serde_json::to_vec(&individual).expect("record json");
    let text = instructions(&zip);
    assert_admitted_exactly(
        &zip,
        &report,
        &[
            (DOCUMENT, fixture!("document.bin")),
            ("acta.pdf.sig", fixture!("holder.sig")),
            ("acta.pdf.tsr", token),
            ("certificado.pem", fixture!("holder.pem")),
            ("ca.pem", fixture!("root.pem")),
            ("crl.pem", fixture!("root.crl.pem")),
            ("tsa-chain.pem", fixture!("root.pem")),
            ("document-consent.bin", fixture!("document-consent.bin")),
            ("document-consent.sig", fixture!("document-consent.sig")),
            (
                "certificate-binding.bin",
                fixture!("certificate-binding.bin"),
            ),
            (
                "certificate-binding.sig",
                fixture!("certificate-binding.sig"),
            ),
            ("binding-root.der", fixture!("root.der")),
            ("binding-crl.der", fixture!("root.crl.der")),
            ("individual-signing.json", &json),
            ("INSTRUCCIONES.md", &text),
        ],
    );
}
