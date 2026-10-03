use application::identity::owner_certificates::{OwnerBindingVerifier, OwnerCertificateError};
use domain::{
    crypto::{CredentialFailure, DocumentHasher},
    identity::{Role, UserId},
    owner_certificates::{BindingStatement, OwnerAccount},
};
use infrastructure::{certificates::InternalRsaOwnerBindingVerifier, RingSha256Hasher};
use uuid::Uuid;

use crate::owner_binding_fixture::fixture;

#[test]
fn application_trait_dispatch_preserves_real_registration_and_neutral_replay_rejection() {
    let fx = fixture();
    let adapter = InternalRsaOwnerBindingVerifier::new();
    let verifier: &dyn OwnerBindingVerifier = &adapter;
    let inspected = verifier.inspect_certificate(&fx.leaf).unwrap();
    assert_eq!(inspected.der, fx.leaf_der);
    assert_eq!(
        inspected.fingerprint,
        RingSha256Hasher.hash_bytes(&fx.leaf_der)
    );

    let canonical = fx.statement.canonical_bytes();
    assert_eq!(canonical.len(), 150);
    let verified = verifier
        .verify_registration(
            &fx.statement,
            &inspected.der,
            &fx.signature,
            &fx.trust,
            fx.at,
        )
        .unwrap();
    assert_eq!(
        verified.statement_digest,
        RingSha256Hasher.hash_bytes(&canonical)
    );
    assert_eq!(verified.certificate, inspected);
    assert_eq!(verified.signature, fx.signature);
    assert_eq!(verified.trust, fx.trust.inspection);
    assert_eq!(verified.checked_at, fx.at);

    // Exercise the application error mapping using the same signature and PKI.
    let another = UserId::from_uuid(Uuid::from_bytes([0x34; 16]));
    let captured = fx.statement.owner();
    let replay = BindingStatement::new(
        OwnerAccount::new(
            another,
            Role::Owner,
            true,
            captured.revision(),
            captured.generation(),
        )
        .unwrap(),
        another,
        *fx.statement.material(),
    )
    .unwrap();
    assert_ne!(replay.canonical_bytes(), canonical);
    assert_eq!(
        verifier
            .verify_registration(&replay, &inspected.der, &fx.signature, &fx.trust, fx.at,)
            .unwrap_err(),
        OwnerCertificateError::Credential(CredentialFailure::InvalidSignature)
    );
}
