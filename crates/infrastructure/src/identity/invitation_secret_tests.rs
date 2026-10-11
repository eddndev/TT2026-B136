//! Binding and confidentiality contracts for persisted invitation payloads.

use application::identity::invitations::{
    InvitationProtectionContext, InvitationProtector, InvitationPurpose,
    MAX_INVITATION_PAYLOAD_BYTES,
};
use application::identity::SecretProtector;
use application::ApplicationError;
use domain::crypto::cipher::SEALED_PAYLOAD_MIN_LEN;
use domain::identity::{InvitationId, UserId};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::{AesGcmInvitationProtector, AesGcmSecretProtector, RingAesGcmCipher};

const KEY: [u8; 32] = [0x42; 32];
const SECRET: &[u8] = b"private-invitation-token-or-pending-mfa-secret";

fn context(purpose: InvitationPurpose) -> InvitationProtectionContext {
    InvitationProtectionContext {
        purpose,
        id: InvitationId::from_uuid(Uuid::from_u128(19)),
        revision: 7,
    }
}

fn protector() -> AesGcmInvitationProtector {
    AesGcmInvitationProtector::new(Zeroizing::new(KEY.to_vec())).unwrap()
}

fn assert_port(error: ApplicationError) {
    assert!(matches!(error, ApplicationError::Port(_)), "{error:?}");
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains(std::str::from_utf8(SECRET).unwrap()));
}

#[test]
fn invitation_secret_round_trips_both_purposes_with_zeroizing_output() {
    let protector = protector();
    for purpose in [InvitationPurpose::Delivery, InvitationPurpose::PendingMfa] {
        let context = context(purpose);
        let protected = protector.protect(context, SECRET).unwrap();
        assert_eq!(protected.len(), SECRET.len() + SEALED_PAYLOAD_MIN_LEN);
        assert!(!protected.windows(SECRET.len()).any(|part| part == SECRET));
        let exposed: Zeroizing<Vec<u8>> = protector.expose(context, &protected).unwrap();
        assert_eq!(exposed.as_slice(), SECRET);
        assert_ne!(protected, protector.protect(context, SECRET).unwrap());
    }
}

#[test]
fn invitation_secret_accepts_the_versioned_persistent_aad_format() {
    let protector = protector();
    for (purpose, discriminator) in [
        (InvitationPurpose::Delivery, 1_u8),
        (InvitationPurpose::PendingMfa, 2_u8),
    ] {
        let context = context(purpose);
        let mut aad = b"qadra:invitation-protection:v1\0".to_vec();
        aad.push(discriminator);
        aad.extend_from_slice(context.id.as_uuid().as_bytes());
        aad.extend_from_slice(&context.revision.to_be_bytes());
        let protected = RingAesGcmCipher::new()
            .seal_with_nonce(&KEY, &[0x13; 12], &aad, SECRET)
            .unwrap()
            .into_bytes();
        assert_eq!(
            protector.expose(context, &protected).unwrap().as_slice(),
            SECRET
        );
    }
}

#[test]
fn invitation_secret_cannot_move_between_purposes_ids_or_revisions() {
    let protector = protector();
    for purpose in [InvitationPurpose::Delivery, InvitationPurpose::PendingMfa] {
        let original = context(purpose);
        let protected = protector.protect(original, SECRET).unwrap();
        let other_purpose = match purpose {
            InvitationPurpose::Delivery => InvitationPurpose::PendingMfa,
            InvitationPurpose::PendingMfa => InvitationPurpose::Delivery,
        };
        for changed in [
            InvitationProtectionContext {
                purpose: other_purpose,
                ..original
            },
            InvitationProtectionContext {
                id: InvitationId::from_uuid(Uuid::from_u128(20)),
                ..original
            },
            InvitationProtectionContext {
                revision: original.revision + 1,
                ..original
            },
        ] {
            assert_port(protector.expose(changed, &protected).unwrap_err());
        }
    }
}

#[test]
fn invitation_secret_rejects_changes_to_nonce_ciphertext_and_tag() {
    let protector = protector();
    let context = context(InvitationPurpose::PendingMfa);
    let protected = protector.protect(context, SECRET).unwrap();
    for index in [0, 12, protected.len() - 1] {
        let mut modified = protected.clone();
        modified[index] ^= 1;
        assert_port(protector.expose(context, &modified).unwrap_err());
    }
    let other_key = AesGcmInvitationProtector::new(Zeroizing::new(vec![0x43; 32])).unwrap();
    assert_port(other_key.expose(context, &protected).unwrap_err());
}

#[test]
fn invitation_secret_has_a_separate_purpose_from_persisted_user_totp() {
    let invitation = protector();
    let totp = AesGcmSecretProtector::new(Zeroizing::new(KEY.to_vec())).unwrap();
    let context = context(InvitationPurpose::PendingMfa);
    let user = UserId::from_uuid(context.id.as_uuid());
    let totp_protected = totp.protect(user, SECRET).unwrap();
    assert_port(invitation.expose(context, &totp_protected).unwrap_err());
    let invitation_protected = invitation.protect(context, SECRET).unwrap();
    assert!(totp.expose(user, &invitation_protected).is_err());
}

#[test]
fn invitation_secret_rejects_invalid_contexts_before_use() {
    let protector = protector();
    let original = context(InvitationPurpose::Delivery);
    let protected = protector.protect(original, SECRET).unwrap();
    for invalid in [
        InvitationProtectionContext {
            id: InvitationId::from_uuid(Uuid::nil()),
            ..original
        },
        InvitationProtectionContext {
            revision: 0,
            ..original
        },
        InvitationProtectionContext {
            revision: i64::MAX as u64 + 1,
            ..original
        },
    ] {
        assert!(matches!(
            protector.protect(invalid, SECRET),
            Err(ApplicationError::InvalidInput(_))
        ));
        assert!(matches!(
            protector.expose(invalid, &protected),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
    let maximum = InvitationProtectionContext {
        revision: i64::MAX as u64,
        ..original
    };
    let protected = protector.protect(maximum, SECRET).unwrap();
    assert_eq!(
        protector.expose(maximum, &protected).unwrap().as_slice(),
        SECRET
    );
}

#[test]
fn invitation_secret_checks_plaintext_and_protected_size_limits() {
    let protector = protector();
    let context = context(InvitationPurpose::Delivery);
    for plaintext in [Vec::new(), vec![0x31; MAX_INVITATION_PAYLOAD_BYTES]] {
        let protected = protector.protect(context, &plaintext).unwrap();
        assert_eq!(
            protector.expose(context, &protected).unwrap().as_slice(),
            plaintext
        );
    }
    let oversized = vec![0x31; MAX_INVITATION_PAYLOAD_BYTES + 1];
    assert!(matches!(
        protector.protect(context, &oversized),
        Err(ApplicationError::InvalidInput(_))
    ));
    let oversized = vec![0; MAX_INVITATION_PAYLOAD_BYTES + SEALED_PAYLOAD_MIN_LEN + 1];
    assert!(matches!(
        protector.expose(context, &oversized),
        Err(ApplicationError::InvalidInput(_))
    ));
}

#[test]
fn invitation_secret_rejects_malformed_payload_without_exposing_its_contents() {
    let protector = protector();
    let context = context(InvitationPurpose::Delivery);
    for protected in [
        Vec::new(),
        vec![0; SEALED_PAYLOAD_MIN_LEN - 1],
        SECRET.to_vec(),
    ] {
        assert_port(protector.expose(context, &protected).unwrap_err());
    }
}

#[test]
fn invitation_secret_requires_aes256_key_without_echoing_key_material() {
    for length in [0, 16, 31, 33] {
        let result = AesGcmInvitationProtector::new(Zeroizing::new(vec![b'K'; length]));
        let error = match result {
            Ok(_) => panic!("invalid key length accepted"),
            Err(error) => error,
        };
        assert!(matches!(error, ApplicationError::InvalidConfiguration(_)));
        assert!(!format!("{error:?} {error}").contains("KKKK"));
    }
}
