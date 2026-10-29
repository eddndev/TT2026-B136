use super::{
    contents, request,
    support::{fixture, Hasher},
    Archive,
};
use application::{
    documents::{IndividualSigningEvidence, ManifestExport, SealedEvidence, SigningManifest},
    evidence::export_individual_with_ports,
};
use domain::{crypto::DocumentHasher, document_signing::DocumentConsent};
use evidence_verify::{
    manifest::{self, Artifact, Manifest, Role, Signer, SignerEntries},
    receipt::{self, Receipt},
};

const DOCUMENT: &[u8] = b"PDF fixture";
const TSA_CHAIN: &[u8] = b"captured-tsa-chain";
const TSA_CRL: &[u8] = b"captured-tsa-crl";

fn role_bytes<'a>(
    sealed: &'a SealedEvidence,
    individual: &'a IndividualSigningEvidence,
) -> Vec<(Role, &'a [u8])> {
    vec![
        (Role::Document, DOCUMENT),
        (Role::DocumentSignature, &sealed.signature),
        (Role::DocumentTimestamp, &sealed.timestamp_token),
        (Role::SignerCertificate, &sealed.signer_certificate_pem),
        (Role::IssuerCertificate, &sealed.issuer_certificate_pem),
        (Role::IssuerCrl, &sealed.crl_pem),
        (Role::TsaChain, TSA_CHAIN),
        (Role::TsaIssuerCrl, TSA_CRL),
        (Role::BindingStatement, &individual.binding_statement),
        (Role::BindingSignature, &individual.binding_signature),
        (Role::BindingRoot, &individual.binding_root_der),
        (Role::BindingCrl, &individual.binding_crl_der),
        (Role::ConsentStatement, &individual.consent_statement),
        (Role::ConsentSignature, &individual.consent_signature),
    ]
}

fn export_of(
    sealed: &SealedEvidence,
    individual: &IndividualSigningEvidence,
    change: impl FnOnce(&mut Manifest),
) -> ManifestExport {
    let consent = DocumentConsent::from_canonical_bytes(&individual.consent_statement).unwrap();
    let mut value = Manifest {
        deployment_id: consent.material().deployment().to_string(),
        case_id: consent.case().as_uuid().to_string(),
        document_id: consent.document().id.as_uuid().to_string(),
        document_version: u64::from(consent.document().version.get()),
        document_sha256: *consent.content_digest().as_bytes(),
        operation_id: consent.operation().to_string(),
        artifacts: role_bytes(sealed, individual)
            .into_iter()
            .map(|(role, bytes)| Artifact {
                entry: manifest::role_entry(role, "contract.pdf"),
                role,
                sha256: *Hasher.hash_bytes(bytes).as_bytes(),
                size: bytes.len() as u64,
            })
            .collect(),
        signers: vec![Signer {
            id: manifest::SIGNER_ID.into(),
            account_id: consent.account().id().as_uuid().to_string(),
            binding_id: consent.material().binding().to_string(),
            binding_registered_at: individual.binding_registered_at,
            entries: SignerEntries::for_document("contract.pdf"),
        }],
    };
    change(&mut value);
    ManifestExport {
        operation: consent.operation(),
        accepted_at_seconds: individual.accepted_at,
        accepted_at_nanoseconds: 123_456_789,
        manifest: SigningManifest {
            manifest: manifest::encode(&value).unwrap(),
            token: b"manifest token".to_vec(),
            tsa_crl_pem: TSA_CRL.to_vec(),
        },
    }
}

fn sealed_with_chain() -> (SealedEvidence, IndividualSigningEvidence) {
    let (_, mut sealed, individual) = fixture();
    sealed.tsa_chain_pem = Some(TSA_CHAIN.to_vec());
    (sealed, individual)
}

#[test]
fn a_manifest_adds_its_token_crl_and_receipt_before_the_instructions() {
    let (sealed, individual) = sealed_with_chain();
    let export = export_of(&sealed, &individual, |_| {});
    let writer = Archive::default();
    let exported = export_individual_with_ports(
        &Hasher,
        &writer,
        &request(&sealed),
        &individual,
        Some(&export),
    )
    .unwrap();
    let entries = writer.0.lock().unwrap();
    let names: Vec<&str> = entries.iter().map(|entry| entry.name()).collect();
    assert_eq!(
        names[names.len() - 6..],
        [
            "individual-signing.json",
            "manifiesto.json",
            "manifiesto.json.tsr",
            "tsa-crl.pem",
            "recibo-aceptacion.json",
            "INSTRUCCIONES.md",
        ]
    );
    assert_eq!(
        contents(&entries, "manifiesto.json"),
        export.manifest.manifest
    );
    assert_eq!(contents(&entries, "manifiesto.json.tsr"), b"manifest token");
    assert_eq!(contents(&entries, "tsa-crl.pem"), TSA_CRL);
    let stored = receipt::parse(contents(&entries, "recibo-aceptacion.json")).unwrap();
    assert_eq!(
        stored,
        Receipt {
            operation_id: export.operation.to_string(),
            manifest_sha256: evidence_verify::certificate::sha256(&export.manifest.manifest),
            accepted_at_seconds: individual.accepted_at,
            accepted_at_nanoseconds: 123_456_789,
        }
    );
    for expected in ["manifiesto.json", "recibo-aceptacion.json", "tsa-crl.pem"] {
        assert!(exported.instructions.contains(expected));
    }
    assert!(!exported.instructions.contains("{{"));
    let again = Archive::default();
    let repeated = export_individual_with_ports(
        &Hasher,
        &again,
        &request(&sealed),
        &individual,
        Some(&export),
    )
    .unwrap();
    assert_eq!(repeated, exported);
}

#[test]
fn a_manifest_that_does_not_describe_the_exported_entries_is_refused() {
    let (sealed, individual) = sealed_with_chain();
    type Change = Box<dyn FnOnce(&mut Manifest)>;
    let cases: Vec<Change> = vec![
        Box::new(|value| value.artifacts[0].size += 1),
        Box::new(|value| value.artifacts[3].sha256[0] ^= 1),
        Box::new(|value| {
            value.artifacts.pop();
        }),
        Box::new(|value| value.operation_id = uuid::Uuid::from_u128(99).to_string()),
        Box::new(|value| value.document_version += 1),
    ];
    for change in cases {
        let export = export_of(&sealed, &individual, change);
        let writer = Archive::default();
        assert!(export_individual_with_ports(
            &Hasher,
            &writer,
            &request(&sealed),
            &individual,
            Some(&export)
        )
        .is_err());
        assert!(writer.0.lock().unwrap().is_empty());
    }
    let mut export = export_of(&sealed, &individual, |_| {});
    export.accepted_at_seconds += 1;
    assert!(export_individual_with_ports(
        &Hasher,
        &Archive::default(),
        &request(&sealed),
        &individual,
        Some(&export)
    )
    .is_err());
    let (_, plain, _) = fixture();
    let export = export_of(&sealed, &individual, |_| {});
    assert!(export_individual_with_ports(
        &Hasher,
        &Archive::default(),
        &request(&plain),
        &individual,
        Some(&export)
    )
    .is_err());
}

#[test]
fn a_package_with_a_manifest_refuses_every_reserved_document_name() {
    let (sealed, individual) = sealed_with_chain();
    let export = export_of(&sealed, &individual, |_| {});
    for name in ["CA.PEM", "manifiesto.json", "Tsa-Crl.pem", "manifiesto"] {
        let writer = Archive::default();
        let request = super::EvidenceRequest {
            document_name: name,
            ..request(&sealed)
        };
        assert!(
            export_individual_with_ports(&Hasher, &writer, &request, &individual, Some(&export))
                .is_err(),
            "{name}"
        );
        assert!(writer.0.lock().unwrap().is_empty());
    }
}
