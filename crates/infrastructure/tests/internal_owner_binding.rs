#[allow(dead_code)]
#[path = "declaration_fixture/mod.rs"]
mod declaration_fixture;
mod owner_binding_fixture;
#[path = "owner_binding_cases/profile.rs"]
mod profile_cases;
#[path = "owner_binding_cases/trust.rs"]
mod trust_cases;

use application::credential_trust::CredentialTrustRevision;
use domain::{
    crypto::{CredentialFailure, DocumentHasher, Sha256Digest, Signature},
    identity::{Role, UserId},
    owner_certificates::{BindingMaterial, BindingRecord, BindingStatement, OwnerAccount},
};
use infrastructure::{certificates::OwnerBindingFailure, RingSha256Hasher};
use uuid::Uuid;

use owner_binding_fixture::{fixture, owner, sign, statement, verify};

#[test]
fn real_partner_and_openssl_signature_verify_the_exact_registration() {
    let fx = fixture();
    let result = verify(&fx.statement, &fx.leaf, &fx.signature, &fx.trust, fx.at).unwrap();
    assert_eq!(fx.statement.canonical_bytes().len(), 150);
    assert_eq!(
        result.statement_digest,
        RingSha256Hasher.hash_bytes(&fx.statement.canonical_bytes())
    );
    assert_eq!(result.signature, fx.signature);
    assert_eq!(result.certificate.der, fx.leaf_der);
    assert_eq!(
        result.certificate.fingerprint,
        RingSha256Hasher.hash_bytes(&fx.leaf_der)
    );
    assert_eq!(result.trust, fx.trust.inspection);
    assert_eq!(result.checked_at, fx.at);
    assert_eq!(
        result.valid_from,
        result
            .certificate
            .summary
            .not_before_unix
            .max(result.trust.valid_from)
    );
    assert_eq!(
        result.valid_until,
        result
            .certificate
            .summary
            .not_after_unix
            .min(result.trust.valid_until)
    );
    assert!(!format!("{result:?}").contains("Synthetic Owner Partner"));
    assert_eq!(
        verify(&fx.statement, &fx.leaf_der, &fx.signature, &fx.trust, fx.at).unwrap(),
        result
    );
}

#[test]
fn registration_signatures_cannot_be_reused_for_another_intent_or_account_state() {
    let fx = fixture();
    let fingerprint = RingSha256Hasher.hash_bytes(&fx.leaf_der);
    let another_user = UserId::from_uuid(Uuid::from_bytes([0x34; 16]));
    let another_account = BindingStatement::new(
        OwnerAccount::new(another_user, Role::Owner, true, 9, 4).unwrap(),
        another_user,
        BindingMaterial::new(
            fx.trust.deployment_id,
            Uuid::from_bytes([0x44; 16]),
            fx.trust.inspection.root_fingerprint,
            fingerprint,
            fx.trust.revision.get(),
        )
        .unwrap(),
    )
    .unwrap();
    for altered in [
        statement(
            &fx.trust,
            fingerprint,
            owner(10, 4),
            Uuid::from_bytes([0x44; 16]),
        ),
        statement(
            &fx.trust,
            fingerprint,
            owner(9, 5),
            Uuid::from_bytes([0x44; 16]),
        ),
        statement(
            &fx.trust,
            fingerprint,
            owner(9, 4),
            Uuid::from_bytes([0x45; 16]),
        ),
        another_account,
    ] {
        assert_eq!(
            verify(&altered, &fx.leaf, &fx.signature, &fx.trust, fx.at).unwrap_err(),
            OwnerBindingFailure::Credential(CredentialFailure::InvalidSignature)
        );
    }
    let record = BindingRecord::registered(fx.statement.clone())
        .withdraw(owner(9, 4), 1)
        .unwrap();
    let withdrawal = record.withdrawal().unwrap().canonical_bytes();
    let mut tampered = fx.signature.as_bytes().to_vec();
    tampered[0] ^= 1;
    for signature in [
        sign(&withdrawal),
        sign(&declaration_fixture::fixture().statement),
        Signature::from_bytes(tampered).unwrap(),
    ] {
        assert_eq!(
            verify(&fx.statement, &fx.leaf, &signature, &fx.trust, fx.at).unwrap_err(),
            OwnerBindingFailure::Credential(CredentialFailure::InvalidSignature)
        );
    }
}

#[test]
fn exact_snapshot_identity_and_recomputed_material_are_mandatory() {
    let fx = fixture();
    let mut deployment = fx.trust.clone();
    deployment.deployment_id = Uuid::from_bytes([0x12; 16]);
    let mut revision = fx.trust.clone();
    revision.revision = CredentialTrustRevision::new(8).unwrap();
    let mut root = fx.trust.clone();
    root.inspection.root_fingerprint = Sha256Digest::from_array([8; 32]);
    let mut digest = fx.trust.clone();
    digest.inspection.crl_digest = Sha256Digest::from_array([9; 32]);
    let mut number = fx.trust.clone();
    number.inspection.crl_number += 1;
    let mut start = fx.trust.clone();
    start.inspection.crl_this_update -= 1;
    let mut end = fx.trust.clone();
    end.inspection.crl_next_update += 1;
    let mut valid_from = fx.trust.clone();
    valid_from.inspection.valid_from -= 1;
    let mut valid_until = fx.trust.clone();
    valid_until.inspection.valid_until += 1;
    for trust in [
        deployment,
        revision,
        root,
        digest,
        number,
        start,
        end,
        valid_from,
        valid_until,
    ] {
        assert_eq!(
            verify(&fx.statement, &fx.leaf, &fx.signature, &trust, fx.at).unwrap_err(),
            OwnerBindingFailure::MaterialMismatch
        );
    }
    let wrong_leaf = statement(
        &fx.trust,
        Sha256Digest::from_array([7; 32]),
        owner(9, 4),
        Uuid::from_bytes([0x44; 16]),
    );
    assert_eq!(
        verify(
            &wrong_leaf,
            &fx.leaf,
            &sign(&wrong_leaf.canonical_bytes()),
            &fx.trust,
            fx.at
        )
        .unwrap_err(),
        OwnerBindingFailure::MaterialMismatch
    );
    for (root, expected) in [
        (true, CredentialFailure::MalformedCertificate),
        (false, CredentialFailure::MalformedCrl),
    ] {
        let mut trust = fx.trust.clone();
        if root {
            trust.inspection.root_der.clear();
        } else {
            trust.inspection.crl_der.clear();
        }
        assert_eq!(
            verify(&fx.statement, &fx.leaf, &fx.signature, &trust, fx.at).unwrap_err(),
            OwnerBindingFailure::Credential(expected)
        );
    }
}
