use std::io::Write;

use application::credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot};
use domain::{
    cases::CaseId,
    crypto::{
        CredentialFailure, DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef,
        InternalDeclarationVerifier, Signature,
    },
    document_signing::{
        DocumentConsent, SignerAccount, SigningBindingMaterial, SigningBindingStatement,
    },
    identity::{Role, UserId},
};
use infrastructure::{
    certificates::{
        DocumentSigningFailure, InternalRsaDeclarationVerifier, InternalRsaDocumentSigningVerifier,
    },
    RingSha256Hasher,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::declaration_fixture::{fixture, openssl};

pub const PDF: &[u8] = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\n%%EOF\n";

pub fn uuid(byte: u8) -> Uuid {
    Uuid::from_bytes([byte; 16])
}

pub fn account(byte: u8) -> SignerAccount {
    SignerAccount::new(UserId::from_uuid(uuid(byte)), Role::Litigator, true, 9, 4).unwrap()
}

pub fn trust() -> CredentialTrustSnapshot {
    let fx = fixture();
    CredentialTrustSnapshot {
        deployment_id: uuid(0x11),
        revision: CredentialTrustRevision::new(7).unwrap(),
        inspection: InternalRsaDeclarationVerifier
            .inspect_trust(&fx.root, &fx.crl, fx.at)
            .unwrap(),
        published_at: OffsetDateTime::from_unix_timestamp(fx.at - 60).unwrap(),
        published_by: "synthetic-admin".into(),
    }
}

pub fn material(certificate: &[u8], trust: &CredentialTrustSnapshot) -> SigningBindingMaterial {
    let leaf = InternalRsaDeclarationVerifier
        .inspect_certificate(certificate)
        .unwrap();
    SigningBindingMaterial::new(
        trust.deployment_id,
        uuid(0x44),
        trust.inspection.root_fingerprint,
        leaf.fingerprint,
        trust.revision.get(),
    )
    .unwrap()
}

pub fn binding(material: SigningBindingMaterial, signer: SignerAccount) -> SigningBindingStatement {
    SigningBindingStatement::new(signer, signer.id(), material).unwrap()
}

pub fn document(byte: u8, version: u32) -> DocumentVersionRef {
    DocumentVersionRef {
        id: DocumentId::from_uuid(uuid(byte)),
        version: DocumentVersion::new(version).unwrap(),
    }
}

pub fn consent(
    material: SigningBindingMaterial,
    signer: SignerAccount,
    document: DocumentVersionRef,
    content: &[u8],
    signature: &Signature,
) -> DocumentConsent {
    DocumentConsent::new(
        signer,
        material,
        uuid(0x66),
        CaseId::from_uuid(uuid(0x77)),
        document,
        RingSha256Hasher.hash_bytes(content),
        RingSha256Hasher.hash_bytes(signature.as_bytes()),
    )
    .unwrap()
}

pub fn sign(bytes: &[u8]) -> Signature {
    let mut input = tempfile::NamedTempFile::new().unwrap();
    input.write_all(bytes).unwrap();
    Signature::from_bytes(openssl(&[
        "dgst",
        "-sha256",
        "-sign",
        fixture()
            .ca
            .join("private/synthetic-declarant.key.pem")
            .to_str()
            .unwrap(),
        input.path().to_str().unwrap(),
    ]))
    .unwrap()
}

pub fn rejected_both(
    certificate: &[u8],
    trust: &CredentialTrustSnapshot,
    expected: CredentialFailure,
) {
    let verifier = InternalRsaDocumentSigningVerifier::new();
    let material = material(certificate, trust);
    let binding = binding(material, account(0x33));
    let pdf_signature = sign(PDF);
    let consent = consent(
        material,
        account(0x33),
        document(0x88, 1),
        PDF,
        &pdf_signature,
    );
    assert_eq!(
        verifier
            .verify_binding(
                &binding,
                certificate,
                &sign(&binding.canonical_bytes()),
                trust,
                fixture().at
            )
            .unwrap_err(),
        DocumentSigningFailure::Credential(expected),
    );
    assert_eq!(
        verifier
            .verify_consent(
                &consent,
                certificate,
                &pdf_signature,
                &sign(&consent.canonical_bytes()),
                trust,
                fixture().at
            )
            .unwrap_err(),
        DocumentSigningFailure::Credential(expected),
    );
}
