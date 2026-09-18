use application::deadline_reevaluation::*;
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    deadlines::{DeadlineId, DeadlineOperationId},
    identity::UserId,
};
use uuid::Uuid;

fn user() -> TrackedAuthor {
    TrackedAuthor::User {
        id: UserId::from_uuid(Uuid::from_u128(4)),
        email: "a".into(),
    }
}
fn receipt() -> TrackedSubmission {
    TrackedSubmission {
        case_id: CaseId::from_uuid(Uuid::from_u128(1)),
        deadline_id: DeadlineId::from_uuid(Uuid::from_u128(2)),
        operation_id: DeadlineOperationId::from_uuid(Uuid::from_u128(3)),
        action: TrackedAction::Register,
        expected_revision: 0,
        review_digest: Sha256Digest::from_array([0x11; 32]),
        observations_digest: Sha256Digest::from_array([0x22; 32]),
        predecessor: None,
        author: user(),
        reason: None,
        cause: None,
    }
}
fn correction() -> TrackedSubmission {
    TrackedSubmission {
        action: TrackedAction::Correct,
        expected_revision: 7,
        predecessor: Some(PredecessorReceipt {
            submission_digest: Sha256Digest::from_array([0x33; 32]),
            capture_digest: Sha256Digest::from_array([0x44; 32]),
        }),
        reason: Some("Changed".into()),
        ..receipt()
    }
}
fn technical() -> TrackedSubmission {
    TrackedSubmission {
        action: TrackedAction::Reevaluate,
        author: TrackedAuthor::Technical {
            service: TechnicalService::DeadlineReevaluator,
            policy_version: 1,
        },
        cause: Some(TechnicalCause::SourceEvent {
            job_id: Uuid::from_u128(5),
            event: SourceEventReference {
                sequence: 6,
                family: DependencyFamily::HearingResult,
                source_id: Uuid::from_u128(7),
                revision: 8,
                case_id: Some(CaseId::from_uuid(Uuid::from_u128(1))),
                hearing_id: Some(Uuid::from_u128(9)),
                operation_id: Uuid::from_u128(10),
            },
        }),
        ..correction()
    }
}
fn roundtrip(value: &TrackedSubmission) -> Vec<u8> {
    let bytes = encode_tracked_submission(value).unwrap();
    assert_eq!(decode_tracked_submission(&bytes).unwrap(), *value);
    bytes
}

#[test]
fn independent_register_vector_has_exact_offsets_and_minimum_size() {
    let mut expected = b"DLTX2".to_vec();
    for id in 1_u128..=3 {
        expected.extend(id.to_be_bytes());
    }
    expected.push(0);
    expected.extend(0_u32.to_be_bytes());
    expected.extend([0x11; 32]);
    expected.extend([0x22; 32]);
    expected.extend([0, 0]);
    expected.extend(4_u128.to_be_bytes());
    expected.extend(1_u64.to_be_bytes());
    expected.extend(b"a");
    expected.extend([0, 0]);
    assert_eq!(expected.len(), 151);
    assert_eq!(roundtrip(&receipt()), expected);
}

#[test]
fn independent_technical_vector_binds_both_predecessor_digests_and_source_event() {
    let mut expected = b"DLTX2".to_vec();
    for id in 1_u128..=3 {
        expected.extend(id.to_be_bytes());
    }
    expected.push(4);
    expected.extend(7_u32.to_be_bytes());
    expected.extend([0x11; 32]);
    expected.extend([0x22; 32]);
    expected.push(1);
    expected.extend([0x33; 32]);
    expected.extend([0x44; 32]);
    expected.extend([1, 0, 0, 1, 1]);
    expected.extend(7_u64.to_be_bytes());
    expected.extend(b"Changed");
    expected.push(1);
    expected.extend(5_u128.to_be_bytes());
    expected.extend(6_u64.to_be_bytes());
    expected.push(2);
    expected.extend(7_u128.to_be_bytes());
    expected.extend(8_u32.to_be_bytes());
    expected.push(1);
    expected.extend(1_u128.to_be_bytes());
    expected.push(1);
    expected.extend(9_u128.to_be_bytes());
    expected.extend(10_u128.to_be_bytes());
    assert_eq!(expected.len(), 303);
    assert_eq!(roundtrip(&technical()), expected);
}

#[test]
fn every_manual_action_and_bootstrap_have_unambiguous_discriminants() {
    for (action, tag) in [
        (TrackedAction::Correct, 1),
        (TrackedAction::SetAttention, 2),
        (TrackedAction::Retire, 3),
    ] {
        let value = TrackedSubmission {
            action,
            ..correction()
        };
        assert_eq!(roundtrip(&value)[53], tag);
    }
    let value = TrackedSubmission {
        cause: Some(TechnicalCause::LegacyBootstrap {
            job_id: Uuid::nil(),
            policy_version: 1,
        }),
        ..technical()
    };
    let bytes = roundtrip(&value);
    assert_eq!(&bytes[207..], &[&[2][..], &[0; 16], &[0, 1]].concat());
}

#[test]
fn utf8_scalar_limits_attain_the_exact_submission_bound_without_normalization() {
    let mut value = correction();
    value.author = TrackedAuthor::User {
        id: UserId::from_uuid(Uuid::nil()),
        email: "\u{1f4c4}".repeat(320),
    };
    value.reason = Some("\u{1f4c4}".repeat(1000));
    let bytes = roundtrip(&value);
    assert_eq!(bytes.len(), 5502);
    assert_eq!(MAX_TRACKED_SUBMISSION_BYTES, bytes.len());
    assert_eq!(&bytes[204..212], &1280_u64.to_be_bytes());
    value.reason = Some("Same\ntext".into());
    assert_eq!(
        decode_tracked_submission(&roundtrip(&value)).unwrap(),
        value
    );
    value.reason = Some("x".repeat(1001));
    assert!(encode_tracked_submission(&value).is_err());
    value.reason = Some("ok".into());
    value.author = TrackedAuthor::User {
        id: UserId::from_uuid(Uuid::nil()),
        email: "x".repeat(321),
    };
    assert!(encode_tracked_submission(&value).is_err());
}

#[test]
fn nil_identifiers_are_present_identities_and_maximum_sequence_is_valid() {
    let mut value = technical();
    value.case_id = CaseId::from_uuid(Uuid::nil());
    value.deadline_id = DeadlineId::from_uuid(Uuid::nil());
    value.operation_id = DeadlineOperationId::from_uuid(Uuid::nil());
    value.expected_revision = u32::MAX - 1;
    let Some(TechnicalCause::SourceEvent { job_id, event }) = &mut value.cause else {
        panic!("source event expected");
    };
    *job_id = Uuid::nil();
    event.source_id = Uuid::nil();
    event.operation_id = Uuid::nil();
    event.case_id = Some(value.case_id);
    event.hearing_id = Some(Uuid::nil());
    event.revision = u32::MAX;
    event.sequence = i64::MAX as u64;
    roundtrip(&value);
}

#[test]
fn all_prefix_truncations_and_trailing_bytes_are_rejected_without_v1_fallback() {
    for value in [receipt(), correction(), technical()] {
        let bytes = roundtrip(&value);
        for length in 0..bytes.len() {
            assert!(
                decode_tracked_submission(&bytes[..length]).is_err(),
                "{length}"
            );
        }
        let mut extended = bytes.clone();
        extended.push(0);
        assert!(decode_tracked_submission(&extended).is_err());
        for version in [b'1', b'3', 0] {
            let mut invalid = bytes.clone();
            invalid[4] = version;
            assert!(decode_tracked_submission(&invalid).is_err());
        }
    }
    assert!(decode_tracked_submission(&vec![0; 5503]).is_err());
}

#[test]
fn unknown_tags_lengths_invalid_utf8_and_text_limits_fail_decode() {
    let register = roundtrip(&receipt());
    for (offset, byte) in [(53, 5), (122, 2), (123, 2), (148, 255), (149, 2), (150, 3)] {
        let mut invalid = register.clone();
        invalid[offset] = byte;
        assert!(decode_tracked_submission(&invalid).is_err(), "{offset}");
    }
    let mut invalid = register;
    invalid[140..148].copy_from_slice(&u64::MAX.to_be_bytes());
    assert!(decode_tracked_submission(&invalid).is_err());
    let bytes = roundtrip(&technical());
    for offset in [188, 190, 191, 207, 232, 253, 270] {
        let mut invalid = bytes.clone();
        invalid[offset] = 255;
        assert!(decode_tracked_submission(&invalid).is_err(), "{offset}");
    }
}

#[test]
fn mismatched_author_action_cause_predecessor_or_revision_cannot_encode() {
    let mut invalid = Vec::new();
    let mut value = receipt();
    value.predecessor = correction().predecessor;
    invalid.push(value);
    let mut value = receipt();
    value.reason = Some("no".into());
    invalid.push(value);
    let mut value = receipt();
    value.expected_revision = 1;
    invalid.push(value);
    let mut value = correction();
    value.predecessor = None;
    invalid.push(value);
    let mut value = correction();
    value.expected_revision = 0;
    invalid.push(value);
    let mut value = correction();
    value.expected_revision = u32::MAX;
    invalid.push(value);
    let mut value = correction();
    value.reason = None;
    invalid.push(value);
    let mut value = correction();
    value.cause = technical().cause;
    invalid.push(value);
    let mut value = technical();
    value.author = user();
    invalid.push(value);
    let mut value = technical();
    value.cause = None;
    invalid.push(value);
    let mut value = technical();
    value.action = TrackedAction::Retire;
    invalid.push(value);
    let mut value = technical();
    value.author = TrackedAuthor::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 2,
    };
    invalid.push(value);
    let mut value = technical();
    value.cause = Some(TechnicalCause::LegacyBootstrap {
        job_id: Uuid::nil(),
        policy_version: 0,
    });
    invalid.push(value);
    for value in invalid {
        assert!(encode_tracked_submission(&value).is_err(), "{value:?}");
    }
}

#[test]
fn event_scope_revision_sequence_and_family_shape_are_validated() {
    for mutate in [
        |event: &mut SourceEventReference| event.sequence = 0,
        |event: &mut SourceEventReference| event.sequence = i64::MAX as u64 + 1,
        |event: &mut SourceEventReference| event.revision = 0,
        |event: &mut SourceEventReference| event.case_id = None,
        |event: &mut SourceEventReference| event.case_id = Some(CaseId::from_uuid(Uuid::nil())),
        |event: &mut SourceEventReference| event.hearing_id = None,
        |event: &mut SourceEventReference| event.family = DependencyFamily::Resolution,
        |event: &mut SourceEventReference| event.family = DependencyFamily::Calendar,
        |event: &mut SourceEventReference| event.family = DependencyFamily::Profile,
    ] {
        let mut value = technical();
        let Some(TechnicalCause::SourceEvent { event, .. }) = &mut value.cause else {
            panic!("source event expected");
        };
        mutate(event);
        assert!(encode_tracked_submission(&value).is_err());
    }
    for family in [
        DependencyFamily::Resolution,
        DependencyFamily::Notification,
        DependencyFamily::Profile,
        DependencyFamily::Calendar,
    ] {
        let mut value = technical();
        let Some(TechnicalCause::SourceEvent { event, .. }) = &mut value.cause else {
            panic!("source event expected");
        };
        event.family = family;
        event.hearing_id = None;
        if family == DependencyFamily::Calendar {
            event.case_id = None;
        }
        roundtrip(&value);
        if family == DependencyFamily::Profile {
            let Some(TechnicalCause::SourceEvent { event, .. }) = &mut value.cause else {
                panic!("source event expected");
            };
            event.case_id = None;
            roundtrip(&value);
        }
    }
}

#[test]
fn empty_or_control_text_cannot_be_encoded() {
    for reason in [
        "",
        "  ",
        "bad\rtext",
        "bad\r\ntext",
        "bad\0text",
        " text",
        "text ",
    ] {
        let value = TrackedSubmission {
            reason: Some(reason.into()),
            ..correction()
        };
        assert!(encode_tracked_submission(&value).is_err());
    }
    for email in ["", "  ", "bad\ntext", "bad\0text", " a", "a "] {
        let value = TrackedSubmission {
            author: TrackedAuthor::User {
                id: UserId::from_uuid(Uuid::nil()),
                email: email.into(),
            },
            ..receipt()
        };
        assert!(encode_tracked_submission(&value).is_err());
    }
}

#[test]
fn decoder_rejects_noncanonical_text_instead_of_trimming_it() {
    let original = roundtrip(&correction());
    for offset in [212, 222, 228] {
        let mut invalid = original.clone();
        invalid[offset] = b' ';
        assert!(decode_tracked_submission(&invalid).is_err(), "{offset}");
    }
}
