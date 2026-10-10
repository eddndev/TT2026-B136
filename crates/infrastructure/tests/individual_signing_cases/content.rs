use domain::crypto::{CredentialFailure, Signature};
use infrastructure::certificates::{DocumentSigningFailure, InternalRsaDocumentSigningVerifier};

use crate::{
    declaration_fixture::fixture,
    individual_signing_fixture::{account, binding, consent, document, material, sign, trust, PDF},
};

#[test]
fn a_signed_consent_cannot_move_to_another_account_document_version_or_content() {
    let fx = fixture();
    let trust = trust();
    let material = material(&fx.leaf, &trust);
    let pdf_signature = sign(PDF);
    let original = consent(
        material,
        account(0x33),
        document(0x88, 1),
        PDF,
        &pdf_signature,
    );
    let signature = sign(&original.canonical_bytes());
    let verifier = InternalRsaDocumentSigningVerifier::new();
    for altered in [
        consent(
            material,
            account(0x34),
            document(0x88, 1),
            PDF,
            &pdf_signature,
        ),
        consent(
            material,
            account(0x33),
            document(0x89, 1),
            PDF,
            &pdf_signature,
        ),
        consent(
            material,
            account(0x33),
            document(0x88, 2),
            PDF,
            &pdf_signature,
        ),
        consent(
            material,
            account(0x33),
            document(0x88, 1),
            b"altered PDF",
            &pdf_signature,
        ),
    ] {
        assert_eq!(
            verifier
                .verify_consent(
                    &altered,
                    &fx.leaf,
                    &pdf_signature,
                    &signature,
                    &trust,
                    fx.at
                )
                .unwrap_err(),
            DocumentSigningFailure::Credential(CredentialFailure::InvalidSignature),
        );
    }
    let original = binding(material, account(0x33));
    let changed = binding(material, account(0x34));
    assert_eq!(
        verifier
            .verify_binding(
                &changed,
                &fx.leaf,
                &sign(&original.canonical_bytes()),
                &trust,
                fx.at
            )
            .unwrap_err(),
        DocumentSigningFailure::Credential(CredentialFailure::InvalidSignature),
    );
}

#[test]
fn valid_consent_never_substitutes_for_verification_of_the_separate_pdf_signature() {
    let fx = fixture();
    let trust = trust();
    let material = material(&fx.leaf, &trust);
    let verifier = InternalRsaDocumentSigningVerifier::new();
    let valid_pdf_signature = sign(PDF);
    let original = consent(
        material,
        account(0x33),
        document(0x88, 1),
        PDF,
        &valid_pdf_signature,
    );
    let mut tampered = valid_pdf_signature.as_bytes().to_vec();
    tampered[0] ^= 1;
    for invalid_pdf in [
        Signature::from_bytes(tampered).unwrap(),
        sign(b"another PDF"),
    ] {
        assert!(verifier
            .verify_consent(
                &original,
                &fx.leaf,
                &invalid_pdf,
                &sign(&original.canonical_bytes()),
                &trust,
                fx.at
            )
            .is_err());
        let matching_hash = consent(
            material,
            account(0x33),
            document(0x88, 1),
            PDF,
            &invalid_pdf,
        );
        assert_eq!(
            verifier
                .verify_consent(
                    &matching_hash,
                    &fx.leaf,
                    &invalid_pdf,
                    &sign(&matching_hash.canonical_bytes()),
                    &trust,
                    fx.at
                )
                .unwrap_err(),
            DocumentSigningFailure::Credential(CredentialFailure::InvalidSignature),
        );
    }
}
