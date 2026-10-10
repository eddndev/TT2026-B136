use application::credential_trust::CredentialTrustRevision;
use domain::{
    crypto::{CredentialFailure, InternalDeclarationVerifier, Sha256Digest},
    document_signing::SigningBindingMaterial,
};
use infrastructure::certificates::{
    DocumentSigningFailure, InternalRsaDeclarationVerifier, InternalRsaDocumentSigningVerifier,
};
use x509_cert::crl::RevokedCert;

use crate::{
    declaration_fixture::{crl, fixture, leaf, signed_certificate, signed_crl, time},
    individual_signing_fixture::{
        account, binding as make_binding, consent as make_consent, document, material,
        rejected_both, sign, trust as snapshot, uuid, PDF,
    },
};

#[test]
fn supplied_snapshot_identity_and_all_recomputed_trust_facts_must_match() {
    let fx = fixture();
    let trusted = snapshot();
    let material = material(&fx.leaf, &trusted);
    let binding = make_binding(material, account(0x33));
    let binding_signature = sign(&binding.canonical_bytes());
    let pdf_signature = sign(PDF);
    let consent = make_consent(
        material,
        account(0x33),
        document(0x88, 1),
        PDF,
        &pdf_signature,
    );
    let consent_signature = sign(&consent.canonical_bytes());
    let verifier = InternalRsaDocumentSigningVerifier::new();
    for field in 0..9 {
        let mut trust = trusted.clone();
        match field {
            0 => trust.deployment_id = uuid(0x12),
            1 => trust.revision = CredentialTrustRevision::new(8).unwrap(),
            2 => trust.inspection.root_fingerprint = Sha256Digest::from_array([1; 32]),
            3 => trust.inspection.crl_digest = Sha256Digest::from_array([2; 32]),
            4 => trust.inspection.crl_number += 1,
            5 => trust.inspection.crl_this_update -= 1,
            6 => trust.inspection.crl_next_update += 1,
            7 => trust.inspection.valid_from -= 1,
            _ => trust.inspection.valid_until += 1,
        }
        assert_eq!(
            verifier
                .verify_binding(&binding, &fx.leaf, &binding_signature, &trust, fx.at)
                .unwrap_err(),
            DocumentSigningFailure::MaterialMismatch,
        );
        assert_eq!(
            verifier
                .verify_consent(
                    &consent,
                    &fx.leaf,
                    &pdf_signature,
                    &consent_signature,
                    &trust,
                    fx.at
                )
                .unwrap_err(),
            DocumentSigningFailure::MaterialMismatch,
        );
    }
    let wrong_leaf = SigningBindingMaterial::new(
        material.deployment(),
        material.binding(),
        material.root(),
        Sha256Digest::from_array([3; 32]),
        material.trust_revision(),
    )
    .unwrap();
    let binding = make_binding(wrong_leaf, account(0x33));
    assert_eq!(
        verifier
            .verify_binding(
                &binding,
                &fx.leaf,
                &sign(&binding.canonical_bytes()),
                &trusted,
                fx.at
            )
            .unwrap_err(),
        DocumentSigningFailure::MaterialMismatch
    );
    let consent = make_consent(
        wrong_leaf,
        account(0x33),
        document(0x88, 1),
        PDF,
        &pdf_signature,
    );
    assert_eq!(
        verifier
            .verify_consent(
                &consent,
                &fx.leaf,
                &pdf_signature,
                &sign(&consent.canonical_bytes()),
                &trusted,
                fx.at
            )
            .unwrap_err(),
        DocumentSigningFailure::MaterialMismatch
    );
}

#[test]
fn revoked_expired_and_not_yet_valid_certificates_reject_both_operations() {
    let fx = fixture();
    let mut revoked = crl();
    revoked.tbs_cert_list.revoked_certificates = Some(vec![RevokedCert {
        serial_number: leaf().tbs_certificate.serial_number,
        revocation_date: revoked.tbs_cert_list.this_update,
        crl_entry_extensions: None,
    }]);
    let mut trust = snapshot();
    trust.inspection = InternalRsaDeclarationVerifier
        .inspect_trust(&fx.root, &signed_crl(revoked), fx.at)
        .unwrap();
    rejected_both(&fx.leaf, &trust, CredentialFailure::Revoked);
    for (before, after, expected) in [
        (fx.at - 1000, fx.at - 1, CredentialFailure::Expired),
        (fx.at + 1, fx.at + 1000, CredentialFailure::NotYetValid),
    ] {
        let mut certificate = leaf();
        certificate.tbs_certificate.validity.not_before = time(before);
        certificate.tbs_certificate.validity.not_after = time(after);
        rejected_both(&signed_certificate(certificate), &snapshot(), expected);
    }
}

#[test]
fn expired_or_future_crl_is_rejected_before_any_signature_is_accepted() {
    let fx = fixture();
    for (start, end, expected) in [
        (fx.at - 1000, fx.at - 1, CredentialFailure::CrlExpired),
        (fx.at + 1, fx.at + 1000, CredentialFailure::CrlNotYetValid),
    ] {
        let mut invalid = crl();
        invalid.tbs_cert_list.this_update = time(start);
        invalid.tbs_cert_list.next_update = Some(time(end));
        let mut trust = snapshot();
        trust.inspection.crl_der = signed_crl(invalid);
        rejected_both(&fx.leaf, &trust, expected);
    }
}
