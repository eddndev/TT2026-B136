#[path = "individual_signing_cases/content.rs"]
mod content_cases;
#[allow(dead_code)]
#[path = "declaration_fixture/mod.rs"]
mod declaration_fixture;
mod individual_signing_fixture;
#[path = "individual_signing_cases/trust.rs"]
mod trust_cases;

use std::fs;

use domain::{
    crypto::{CredentialFailure, DocumentHasher},
    identity::Role,
    owner_certificates::{BindingMaterial, BindingStatement, OwnerAccount},
};
use infrastructure::{
    certificates::{DocumentSigningFailure, InternalRsaDocumentSigningVerifier},
    RingSha256Hasher,
};

use declaration_fixture::{fixture, script};
use individual_signing_fixture::{account, binding, consent, document, material, sign, trust, PDF};

#[test]
fn openssl_public_proofs_preserve_exact_binding_consent_and_captured_trust() {
    let fx = fixture();
    let trust = trust();
    let verifier = InternalRsaDocumentSigningVerifier::new();
    let material = material(&fx.leaf, &trust);
    let binding = binding(material, account(0x33));
    let binding_signature = sign(&binding.canonical_bytes());
    let pdf_signature = sign(PDF);
    let consent = consent(
        material,
        account(0x33),
        document(0x88, 1),
        PDF,
        &pdf_signature,
    );
    let consent_signature = sign(&consent.canonical_bytes());
    for certificate in [&fx.leaf, &fx.leaf_der] {
        let registration = verifier
            .verify_binding(&binding, certificate, &binding_signature, &trust, fx.at)
            .unwrap();
        let signed = verifier
            .verify_consent(
                &consent,
                certificate,
                &pdf_signature,
                &consent_signature,
                &trust,
                fx.at,
            )
            .unwrap();
        for (check, bytes, signature) in [
            (
                registration,
                binding.canonical_bytes().to_vec(),
                &binding_signature,
            ),
            (
                signed,
                consent.canonical_bytes().to_vec(),
                &consent_signature,
            ),
        ] {
            assert_eq!(check.statement_digest, RingSha256Hasher.hash_bytes(&bytes));
            assert_eq!(&check.signature, signature);
            assert_eq!(check.certificate.der, fx.leaf_der);
            assert_eq!(check.certificate.fingerprint, material.leaf());
            assert_eq!(check.trust, trust.inspection);
            assert_eq!(check.checked_at, fx.at);
            assert_eq!(
                check.valid_from,
                check
                    .certificate
                    .summary
                    .not_before_unix
                    .max(trust.inspection.valid_from)
            );
            assert_eq!(
                check.valid_until,
                check
                    .certificate
                    .summary
                    .not_after_unix
                    .min(trust.inspection.valid_until)
            );
        }
    }
}

#[test]
fn signing_intake_rejects_partner_profiles_and_private_or_combined_material() {
    let fx = fixture();
    script("issue-cert.sh", &fx.ca, &["Synthetic Unsupported Partner"]);
    let verifier = InternalRsaDocumentSigningVerifier::new();
    let partner = fs::read(fx.ca.join("certs/synthetic-unsupported-partner.crt.pem")).unwrap();
    assert_eq!(
        verifier.inspect_certificate(&partner).unwrap_err(),
        CredentialFailure::UnsupportedCertificate
    );
    let trust = trust();
    let material = material(&fx.leaf, &trust);
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
                &partner,
                &sign(&binding.canonical_bytes()),
                &trust,
                fx.at
            )
            .unwrap_err(),
        DocumentSigningFailure::Credential(CredentialFailure::UnsupportedCertificate)
    );
    assert_eq!(
        verifier
            .verify_consent(
                &consent,
                &partner,
                &pdf_signature,
                &sign(&consent.canonical_bytes()),
                &trust,
                fx.at
            )
            .unwrap_err(),
        DocumentSigningFailure::Credential(CredentialFailure::UnsupportedCertificate)
    );
    let private = fs::read(fx.ca.join("private/synthetic-declarant.key.pem")).unwrap();
    for bytes in [
        private.clone(),
        [fx.leaf.clone(), private].concat(),
        vec![0; 16385],
    ] {
        assert!(verifier.inspect_certificate(&bytes).is_err());
    }
}

#[test]
fn owner_binding_and_document_consent_signatures_cannot_be_reused_across_purposes() {
    let fx = fixture();
    let trust = trust();
    let material = material(&fx.leaf, &trust);
    let binding = binding(material, account(0x33));
    let owner = OwnerAccount::new(account(0x33).id(), Role::Owner, true, 9, 4).unwrap();
    let owner_statement = BindingStatement::new(
        owner,
        owner.id(),
        BindingMaterial::new(
            material.deployment(),
            material.binding(),
            material.root(),
            material.leaf(),
            material.trust_revision(),
        )
        .unwrap(),
    )
    .unwrap();
    let pdf_signature = sign(PDF);
    let consent = consent(
        material,
        account(0x33),
        document(0x88, 1),
        PDF,
        &pdf_signature,
    );
    let verifier = InternalRsaDocumentSigningVerifier::new();
    for bytes in [
        owner_statement.canonical_bytes().to_vec(),
        consent.canonical_bytes().to_vec(),
    ] {
        assert_eq!(
            verifier
                .verify_binding(&binding, &fx.leaf, &sign(&bytes), &trust, fx.at)
                .unwrap_err(),
            DocumentSigningFailure::Credential(CredentialFailure::InvalidSignature),
        );
    }
    for bytes in [
        owner_statement.canonical_bytes().to_vec(),
        binding.canonical_bytes().to_vec(),
        PDF.to_vec(),
    ] {
        assert_eq!(
            verifier
                .verify_consent(
                    &consent,
                    &fx.leaf,
                    &pdf_signature,
                    &sign(&bytes),
                    &trust,
                    fx.at
                )
                .unwrap_err(),
            DocumentSigningFailure::Credential(CredentialFailure::InvalidSignature),
        );
    }
}
