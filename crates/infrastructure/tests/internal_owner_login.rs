#[allow(dead_code)]
#[path = "declaration_fixture/mod.rs"]
mod declaration_fixture;
#[allow(dead_code)]
mod owner_binding_fixture;

use application::credential_trust::CredentialTrustRevision;
use domain::{
    crypto::{CredentialFailure, DocumentHasher, Sha256Digest, Signature},
    identity::{Role, UserId},
    owner_certificate_login::{LoginAccount, LoginNonce, LoginStatement},
    owner_certificates::{BindingMaterial, BindingRecord},
};
use infrastructure::{
    certificates::{InternalRsaOwnerLoginVerifier, OwnerLoginFailure},
    RingSha256Hasher,
};
use owner_binding_fixture::{fixture, sign};
use uuid::Uuid;

fn statement() -> LoginStatement {
    let fx = fixture();
    LoginStatement::new(
        LoginAccount::new(
            UserId::from_uuid(Uuid::from_bytes([0x33; 16])),
            Role::Owner,
            true,
            4,
        )
        .unwrap(),
        *fx.statement.material(),
        LoginNonce::from_bytes(&[0x76; 32]).unwrap(),
        fx.at - 10,
        fx.at + 290,
    )
    .unwrap()
}

#[test]
fn openssl_partner_proof_verifies_only_the_exact_typed_login() {
    let fx = fixture();
    let statement = statement();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), statement.canonical_bytes()).unwrap();
    let signature = Signature::from_bytes(declaration_fixture::openssl(&[
        "dgst",
        "-sha256",
        "-sign",
        declaration_fixture::fixture()
            .ca
            .join("private/synthetic-owner-partner.key.pem")
            .to_str()
            .unwrap(),
        file.path().to_str().unwrap(),
    ]))
    .unwrap();
    let verifier = InternalRsaOwnerLoginVerifier::new();
    let check = verifier
        .verify_login(&statement, &fx.leaf, &signature, &fx.trust, fx.at)
        .unwrap();
    assert_eq!(
        check.statement_digest,
        RingSha256Hasher.hash_bytes(&statement.canonical_bytes())
    );
    assert_eq!(check.signature, signature);
    assert_eq!(check.certificate.der, fx.leaf_der);
    assert_eq!(check.trust, fx.trust.inspection);
    assert_eq!(check.checked_at, fx.at);
    assert_eq!(
        check.valid_until,
        check
            .certificate
            .summary
            .not_after_unix
            .min(check.trust.valid_until)
    );
    assert!(check.valid_until > statement.expires_at_unix_seconds());
    assert_eq!(
        verifier
            .verify_login(&statement, &fx.leaf_der, &signature, &fx.trust, fx.at)
            .unwrap(),
        check
    );
}

#[test]
fn challenge_expiry_is_exclusive_and_independent_from_certificate_validity() {
    let fx = fixture();
    let statement = statement();
    let signature = sign(&statement.canonical_bytes());
    let verifier = InternalRsaOwnerLoginVerifier::new();
    for at in [
        statement.issued_at_unix_seconds(),
        statement.expires_at_unix_seconds() - 1,
    ] {
        assert!(verifier
            .verify_login(&statement, &fx.leaf, &signature, &fx.trust, at)
            .is_ok());
    }
    for at in [
        -1,
        statement.issued_at_unix_seconds() - 1,
        statement.expires_at_unix_seconds(),
    ] {
        assert_eq!(
            verifier
                .verify_login(&statement, &fx.leaf, &signature, &fx.trust, at)
                .unwrap_err(),
            OwnerLoginFailure::InvalidWindow
        );
    }
}

#[test]
fn registration_withdrawal_and_changed_login_proofs_are_not_interchangeable() {
    let fx = fixture();
    let statement = statement();
    let terminal = BindingRecord::registered(fx.statement.clone())
        .withdraw(owner_binding_fixture::owner(9, 4), 1)
        .unwrap();
    let mut changed = statement.canonical_bytes();
    changed[134] ^= 1;
    let mut generation = statement.canonical_bytes();
    generation[85] ^= 1;
    for signature in [
        fx.signature.clone(),
        sign(&terminal.withdrawal().unwrap().canonical_bytes()),
        sign(&changed),
        sign(&generation),
        sign(&declaration_fixture::fixture().statement),
    ] {
        assert_eq!(
            InternalRsaOwnerLoginVerifier::new()
                .verify_login(&statement, &fx.leaf, &signature, &fx.trust, fx.at,)
                .unwrap_err(),
            OwnerLoginFailure::Credential(CredentialFailure::InvalidSignature)
        );
    }
}

#[test]
fn captured_trust_and_leaf_must_match_recomputed_public_evidence() {
    let fx = fixture();
    let statement = statement();
    let signature = sign(&statement.canonical_bytes());
    let mut deployment = fx.trust.clone();
    deployment.deployment_id = Uuid::from_bytes([0x12; 16]);
    let mut revision = fx.trust.clone();
    revision.revision = CredentialTrustRevision::new(8).unwrap();
    let mut inspection = fx.trust.clone();
    inspection.inspection.crl_number += 1;
    for trust in [deployment, revision, inspection] {
        assert_eq!(
            InternalRsaOwnerLoginVerifier::new()
                .verify_login(&statement, &fx.leaf, &signature, &trust, fx.at,)
                .unwrap_err(),
            OwnerLoginFailure::MaterialMismatch
        );
    }
    let wrong = LoginStatement::new(
        *statement.owner(),
        BindingMaterial::new(
            fx.trust.deployment_id,
            statement.material().binding(),
            fx.trust.inspection.root_fingerprint,
            Sha256Digest::from_array([0; 32]),
            fx.trust.revision.get(),
        )
        .unwrap(),
        *statement.nonce(),
        fx.at,
        fx.at + 100,
    )
    .unwrap();
    assert_eq!(
        InternalRsaOwnerLoginVerifier::new()
            .verify_login(
                &wrong,
                &fx.leaf,
                &sign(&wrong.canonical_bytes()),
                &fx.trust,
                fx.at,
            )
            .unwrap_err(),
        OwnerLoginFailure::MaterialMismatch
    );
}

#[test]
fn revoked_partner_and_declaration_profile_cannot_authenticate() {
    let fx = fixture();
    let statement = statement();
    let signature = sign(&statement.canonical_bytes());
    let mut crl = declaration_fixture::crl();
    crl.tbs_cert_list.revoked_certificates = Some(vec![x509_cert::crl::RevokedCert {
        serial_number: owner_binding_fixture::partner()
            .tbs_certificate
            .serial_number,
        revocation_date: crl.tbs_cert_list.this_update,
        crl_entry_extensions: None,
    }]);
    let trust = owner_binding_fixture::trust_with(
        &fx.trust.inspection.root_der,
        &declaration_fixture::signed_crl(crl),
    );
    assert_eq!(
        InternalRsaOwnerLoginVerifier::new()
            .verify_login(&statement, &fx.leaf, &signature, &trust, fx.at,)
            .unwrap_err(),
        OwnerLoginFailure::Credential(CredentialFailure::Revoked)
    );
    assert!(InternalRsaOwnerLoginVerifier::new()
        .verify_login(
            &statement,
            &declaration_fixture::fixture().leaf,
            &signature,
            &fx.trust,
            fx.at,
        )
        .is_err());
}

#[test]
fn malformed_public_material_and_wrong_signature_size_fail_neutrally() {
    let fx = fixture();
    let statement = statement();
    let signature = sign(&statement.canonical_bytes());
    let verifier = InternalRsaOwnerLoginVerifier::new();
    for leaf in [b"not a certificate".to_vec(), vec![0; 16385]] {
        assert!(verifier
            .verify_login(&statement, &leaf, &signature, &fx.trust, fx.at)
            .is_err());
    }
    for size in [1, 383, 385] {
        let short = Signature::from_bytes(vec![0; size]).unwrap();
        assert_eq!(
            verifier
                .verify_login(&statement, &fx.leaf, &short, &fx.trust, fx.at)
                .unwrap_err(),
            OwnerLoginFailure::Credential(CredentialFailure::InvalidSignature)
        );
    }
    let error = verifier
        .verify_login(&statement, &[], &signature, &fx.trust, fx.at)
        .unwrap_err();
    assert!(!error.to_string().contains("Synthetic"));
}
