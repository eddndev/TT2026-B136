mod case_report_protection_support;

use application::{case_reports::*, ApplicationError};
use case_report_protection_support::*;
use domain::crypto::{
    cipher::SEALED_PAYLOAD_MIN_LEN, KeyManager, SealedPayload, Sha256Digest, WrappedDek,
};
use infrastructure::{CaseReportEnvelopeProtector, EnvelopeKeyManager, RingAesGcmCipher};
use std::sync::{atomic::Ordering, Arc};
use zeroize::Zeroizing;

#[test]
fn accepts_only_a_32_byte_key_encryption_key() {
    for length in [0, 16, 31, 33, 64] {
        let result = CaseReportEnvelopeProtector::new(
            Arc::new(RingAesGcmCipher::new()),
            Arc::new(EnvelopeKeyManager::new()),
            vec![0x52; length],
        );
        assert!(matches!(
            result,
            Err(ApplicationError::InvalidConfiguration(_))
        ));
    }
    assert!(CaseReportEnvelopeProtector::new(
        Arc::new(RingAesGcmCipher::new()),
        Arc::new(EnvelopeKeyManager::new()),
        KEK.to_vec(),
    )
    .is_ok());
}

#[test]
fn all_three_payload_kinds_restore_the_exact_original_bytes_after_reopening() {
    let bytes = b"report\0\xff\xc3\xb1\nprivate bytes";
    for kind in [
        CaseReportPayloadKind::Snapshot,
        CaseReportPayloadKind::Pdf,
        CaseReportPayloadKind::Csv,
    ] {
        let context = context(kind, bytes);
        let original = protector();
        let encrypted = original.seal(context, bytes).unwrap();
        drop(original);
        assert!(!encrypted
            .payload
            .as_bytes()
            .windows(bytes.len())
            .any(|slice| slice == bytes));
        assert_eq!(protector().open(context, &encrypted).unwrap(), bytes);
    }
}

#[test]
fn repeated_identical_payloads_receive_fresh_data_keys_and_envelopes() {
    let protector = protector();
    let bytes = b"same captured report";
    let context = context(CaseReportPayloadKind::Snapshot, bytes);
    let first = protector.seal(context, bytes).unwrap();
    let second = protector.seal(context, bytes).unwrap();
    assert_ne!(first.payload, second.payload);
    assert_ne!(first.wrapped_dek, second.wrapped_dek);
    let keys = EnvelopeKeyManager::new();
    let first_key = Zeroizing::new(keys.unwrap_dek(&KEK, &first.wrapped_dek).unwrap());
    let second_key = Zeroizing::new(keys.unwrap_dek(&KEK, &second.wrapped_dek).unwrap());
    assert_ne!(*first_key, *second_key);
}

#[test]
fn report_id_payload_kind_and_declared_digest_are_authenticated_context() {
    let protector = protector();
    let bytes = b"captured content";
    let correct = context(CaseReportPayloadKind::Snapshot, bytes);
    let encrypted = protector.seal(correct, bytes).unwrap();
    let mut changed = [correct; 4];
    changed[0].report_id = CaseReportId::new();
    changed[1].kind = CaseReportPayloadKind::Pdf;
    changed[2].kind = CaseReportPayloadKind::Csv;
    changed[3].plaintext_digest = Sha256Digest::from_array([0; 32]);
    for context in changed {
        assert!(protector.open(context, &encrypted).is_err());
    }
    assert_eq!(protector.open(correct, &encrypted).unwrap(), bytes);
}

#[test]
fn key_encryption_key_swaps_wrapped_key_swaps_and_ciphertext_tampering_are_rejected() {
    let protector = protector();
    let bytes = b"report tamper fixture";
    let context = context(CaseReportPayloadKind::Pdf, bytes);
    let original = protector.seal(context, bytes).unwrap();
    let other = protector.seal(context, bytes).unwrap();
    let wrong_kek = CaseReportEnvelopeProtector::new(
        Arc::new(RingAesGcmCipher::new()),
        Arc::new(EnvelopeKeyManager::new()),
        vec![0x53; 32],
    )
    .unwrap();
    assert!(wrong_kek.open(context, &original).is_err());
    let mut swapped = original.clone();
    swapped.wrapped_dek = other.wrapped_dek;
    assert!(protector.open(context, &swapped).is_err());
    for offset in [0, 12, original.payload.as_bytes().len() - 1] {
        let mut changed = original.clone();
        let mut bytes = changed.payload.as_bytes().to_vec();
        bytes[offset] ^= 1;
        changed.payload = SealedPayload::from_bytes(bytes).unwrap();
        assert!(protector.open(context, &changed).is_err());
    }
    let mut changed = original.clone();
    let mut key = changed.wrapped_dek.as_bytes().to_vec();
    key[12] ^= 1;
    changed.wrapped_dek = WrappedDek::from_bytes(key).unwrap();
    assert!(protector.open(context, &changed).is_err());
}

#[test]
fn false_plaintext_digest_is_rejected_before_generating_or_sealing_any_key() {
    let (protector, calls) = counted(false);
    let context = context(CaseReportPayloadKind::Csv, b"different bytes");
    assert!(matches!(
        protector.seal(context, b"actual bytes"),
        Err(ApplicationError::CaseReport(
            CaseReportError::StoredInconsistent(_)
        ))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn plaintext_returned_by_the_cipher_is_checked_against_the_recorded_digest() {
    let (protector, calls) = counted(true);
    let bytes = b"bounded original";
    let context = context(CaseReportPayloadKind::Csv, bytes);
    let encrypted = protector.seal(context, bytes).unwrap();
    calls.store(0, Ordering::SeqCst);
    assert!(matches!(
        protector.open(context, &encrypted),
        Err(ApplicationError::CaseReport(
            CaseReportError::StoredInconsistent(_)
        ))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn plaintext_caps_are_checked_before_entering_either_cryptographic_port() {
    let (protector, calls) = counted(false);
    for (kind, maximum) in limits() {
        let bytes = vec![0x41; maximum + 1];
        let context = context(kind, &bytes);
        assert!(matches!(
            protector.seal(context, &bytes),
            Err(ApplicationError::CaseReport(
                CaseReportError::CapacityExceeded
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn encrypted_payload_and_wrapped_key_caps_are_checked_before_decryption() {
    let (protector, calls) = counted(false);
    for (kind, maximum) in limits() {
        let context = context(kind, b"unused");
        let encrypted = ProtectedCaseReportPayload {
            wrapped_dek: WrappedDek::from_bytes(vec![0; 32 + SEALED_PAYLOAD_MIN_LEN]).unwrap(),
            payload: SealedPayload::from_bytes(vec![0; maximum + SEALED_PAYLOAD_MIN_LEN + 1])
                .unwrap(),
        };
        assert!(matches!(
            protector.open(context, &encrypted),
            Err(ApplicationError::CaseReport(
                CaseReportError::CapacityExceeded
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
    let context = context(CaseReportPayloadKind::Snapshot, b"unused");
    for size in [
        SEALED_PAYLOAD_MIN_LEN,
        31 + SEALED_PAYLOAD_MIN_LEN,
        33 + SEALED_PAYLOAD_MIN_LEN,
    ] {
        let encrypted = ProtectedCaseReportPayload {
            wrapped_dek: WrappedDek::from_bytes(vec![0; size]).unwrap(),
            payload: SealedPayload::from_bytes(vec![0; SEALED_PAYLOAD_MIN_LEN]).unwrap(),
        };
        assert!(matches!(
            protector.open(context, &encrypted),
            Err(ApplicationError::CaseReport(
                CaseReportError::StoredInconsistent(_)
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn exact_snapshot_and_artifact_caps_are_accepted_in_both_directions() {
    let protector = protector();
    for (kind, maximum) in limits() {
        let bytes = vec![0x43; maximum];
        let context = context(kind, &bytes);
        let encrypted = protector.seal(context, &bytes).unwrap();
        assert_eq!(
            encrypted.payload.as_bytes().len(),
            maximum + SEALED_PAYLOAD_MIN_LEN
        );
        assert_eq!(protector.open(context, &encrypted).unwrap(), bytes);
    }
}

fn limits() -> [(CaseReportPayloadKind, usize); 3] {
    [
        (CaseReportPayloadKind::Snapshot, MAX_REPORT_SNAPSHOT_BYTES),
        (CaseReportPayloadKind::Pdf, MAX_REPORT_ARTIFACT_BYTES),
        (CaseReportPayloadKind::Csv, MAX_REPORT_ARTIFACT_BYTES),
    ]
}

#[test]
fn nil_report_identity_is_rejected_before_sealing_or_opening() {
    let (protector, calls) = counted(false);
    let mut context = context(CaseReportPayloadKind::Csv, b"report bytes");
    context.report_id = CaseReportId::from_uuid(uuid::Uuid::nil());
    assert!(matches!(
        protector.seal(context, b"report bytes"),
        Err(ApplicationError::CaseReport(
            CaseReportError::StoredInconsistent(_)
        ))
    ));
    let encrypted = ProtectedCaseReportPayload {
        wrapped_dek: WrappedDek::from_bytes(vec![0; 32 + SEALED_PAYLOAD_MIN_LEN]).unwrap(),
        payload: SealedPayload::from_bytes(vec![0; SEALED_PAYLOAD_MIN_LEN]).unwrap(),
    };
    assert!(matches!(
        protector.open(context, &encrypted),
        Err(ApplicationError::CaseReport(
            CaseReportError::StoredInconsistent(_)
        ))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn stored_envelope_aad_version_has_a_fixed_cross_release_encoding() {
    use domain::crypto::AuthenticatedCipher;
    let protector = protector();
    let plaintext = b"report compatibility fixture";
    for (kind, tag) in [
        (CaseReportPayloadKind::Snapshot, 1),
        (CaseReportPayloadKind::Pdf, 2),
        (CaseReportPayloadKind::Csv, 3),
    ] {
        let mut context = context(kind, plaintext);
        context.report_id = CaseReportId::from_uuid(uuid::Uuid::from_bytes([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        ]));
        let protected = protector.seal(context, plaintext).unwrap();
        let key = Zeroizing::new(
            EnvelopeKeyManager::new()
                .unwrap_dek(&KEK, &protected.wrapped_dek)
                .unwrap(),
        );
        let mut aad = b"tt-case-report-envelope-v1\0".to_vec();
        aad.extend_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        aad.push(tag);
        aad.extend_from_slice(context.plaintext_digest.as_bytes());
        assert_eq!(
            RingAesGcmCipher::new()
                .open(&key, &aad, &protected.payload)
                .unwrap(),
            plaintext
        );
    }
}
