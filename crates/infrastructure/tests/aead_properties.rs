//! Behavioral suites for the AES-256-GCM cipher adapter: round-trip
//! property, single-byte mutation detection, AAD binding, and nonce
//! freshness.

use std::collections::HashSet;

use domain::crypto::cipher::{document_aad, AuthenticatedCipher, SealedPayload, AES_GCM_NONCE_LEN};
use domain::crypto::document::{DocumentId, DocumentVersion};
use domain::DomainError;
use infrastructure::RingAesGcmCipher;
use proptest::prelude::*;
use uuid::Uuid;

const KEY: [u8; 32] = [0x6f; 32];

proptest! {
    /// Whatever the key, AAD, and plaintext, opening a sealed payload with
    /// the same key and AAD returns the plaintext.
    #[test]
    fn seal_open_round_trips(
        key in prop::collection::vec(any::<u8>(), 32),
        aad in prop::collection::vec(any::<u8>(), 0..64),
        plaintext in prop::collection::vec(any::<u8>(), 0..1024),
    ) {
        let cipher = RingAesGcmCipher::new();
        let sealed = cipher.seal(&key, &aad, &plaintext).unwrap();
        let opened = cipher.open(&key, &aad, &sealed).unwrap();
        prop_assert_eq!(opened, plaintext);
    }
}

/// Flipping any single byte of the sealed payload (nonce, ciphertext, or
/// tag) must make `open` fail with the opaque authentication error.
#[test]
fn any_single_byte_flip_breaks_authentication() {
    let cipher = RingAesGcmCipher::new();
    let aad = b"binding";
    let sealed = cipher.seal(&KEY, aad, b"a plaintext long enough to matter");
    let bytes = sealed.unwrap().into_bytes();

    for index in 0..bytes.len() {
        let mut mutated = bytes.clone();
        mutated[index] ^= 0x01;
        let payload = SealedPayload::from_bytes(mutated).unwrap();
        let err = cipher.open(&KEY, aad, &payload).unwrap_err();
        assert_eq!(
            err,
            DomainError::AuthenticationFailed,
            "byte {index} flipped but open did not fail with the opaque error"
        );
    }
}

/// Opening with a different key must fail with the same opaque error.
#[test]
fn wrong_key_breaks_authentication() {
    let cipher = RingAesGcmCipher::new();
    let sealed = cipher.seal(&KEY, b"", b"secret").unwrap();
    let mut wrong_key = KEY;
    wrong_key[0] ^= 0x01;
    let err = cipher.open(&wrong_key, b"", &sealed).unwrap_err();
    assert_eq!(err, DomainError::AuthenticationFailed);
}

/// A payload sealed for one document id must not open under another id.
#[test]
fn different_document_id_breaks_authentication() {
    let cipher = RingAesGcmCipher::new();
    let version = DocumentVersion::initial();
    let id = DocumentId::from_uuid(Uuid::from_bytes([0x21; 16]));
    let other_id = DocumentId::from_uuid(Uuid::from_bytes([0x22; 16]));

    let sealed = cipher
        .seal(&KEY, &document_aad(id, version), b"contract text")
        .unwrap();
    let err = cipher
        .open(&KEY, &document_aad(other_id, version), &sealed)
        .unwrap_err();
    assert_eq!(err, DomainError::AuthenticationFailed);
}

/// A payload sealed for one version must not open as the next or previous
/// version, which is what blocks replaying an older ciphertext.
#[test]
fn version_off_by_one_breaks_authentication() {
    let cipher = RingAesGcmCipher::new();
    let id = DocumentId::from_uuid(Uuid::from_bytes([0x33; 16]));
    let version = DocumentVersion::new(7).unwrap();

    let sealed = cipher
        .seal(&KEY, &document_aad(id, version), b"contract text")
        .unwrap();
    for other in [
        DocumentVersion::new(6).unwrap(),
        DocumentVersion::new(8).unwrap(),
    ] {
        let err = cipher
            .open(&KEY, &document_aad(id, other), &sealed)
            .unwrap_err();
        assert_eq!(err, DomainError::AuthenticationFailed);
    }
}

/// Ten thousand seal operations must never repeat a nonce: each call draws
/// fresh CSPRNG output instead of reusing or incrementing state visibly.
#[test]
fn nonces_are_distinct_across_ten_thousand_seals() {
    let cipher = RingAesGcmCipher::new();
    let mut seen = HashSet::new();
    for _ in 0..10_000 {
        let sealed = cipher.seal(&KEY, b"", b"").unwrap();
        let nonce: [u8; AES_GCM_NONCE_LEN] = sealed.nonce().try_into().unwrap();
        assert!(seen.insert(nonce), "a nonce was repeated across seals");
    }
    assert_eq!(seen.len(), 10_000);
}
