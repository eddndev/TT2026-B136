use super::receipt_projection::{author, version};
use crate::error::ApiError;
use application::{
    deadline_reevaluation::{
        DependencyFamily, PredecessorReceipt, SourceEventReference, TechnicalCause,
        TechnicalService,
    },
    deadlines::{DeadlineActorSnapshot, DeadlineReceiptVersion, DeadlineTrackedReceipt},
};
use axum::{http::StatusCode, response::IntoResponse};
use domain::{cases::CaseId, crypto::Sha256Digest, identity::UserId};
use serde_json::{json, Value};
use uuid::Uuid;

fn case() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}

fn digest(byte: u8) -> Sha256Digest {
    Sha256Digest::from_bytes(&[byte; 32]).unwrap()
}

fn tracked(cause: Option<TechnicalCause>) -> DeadlineReceiptVersion {
    DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
        observations_digest: digest(1),
        predecessor: Some(PredecessorReceipt {
            submission_digest: digest(2),
            capture_digest: digest(3),
        }),
        cause,
    })
}

fn event(
    family: DependencyFamily,
    case_id: Option<CaseId>,
    hearing_id: Option<Uuid>,
) -> SourceEventReference {
    SourceEventReference {
        sequence: 9_007_199_254_740_993,
        family,
        source_id: Uuid::from_u128(2),
        revision: 7,
        case_id,
        hearing_id,
        operation_id: Uuid::from_u128(3),
    }
}

fn event_version(event: SourceEventReference) -> DeadlineReceiptVersion {
    tracked(Some(TechnicalCause::SourceEvent {
        job_id: Uuid::nil(),
        event,
    }))
}

fn expected_tracked(cause: Value) -> Value {
    json!({
        "kind": "v2",
        "observations_digest": "01".repeat(32),
        "predecessor": {
            "submission_digest": "02".repeat(32),
            "capture_digest": "03".repeat(32)
        },
        "cause": cause
    })
}

fn assert_internal(result: Result<Value, ApiError>) {
    assert_eq!(
        result.unwrap_err().into_response().status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[test]
fn user_author_has_a_tag_and_preserves_a_zero_uuid() {
    let value = DeadlineActorSnapshot::User {
        id: UserId::from_uuid(Uuid::nil()),
        email: "owner@example.test".into(),
    };
    assert_eq!(
        author(&value).unwrap(),
        json!({"kind": "user", "id": Uuid::nil(), "email": "owner@example.test"})
    );
}

#[test]
fn user_author_preserves_the_canonical_unicode_scalar_limit() {
    let email = "\u{e9}".repeat(320);
    let value = DeadlineActorSnapshot::User {
        id: UserId::from_uuid(Uuid::nil()),
        email: email.clone(),
    };
    assert_eq!(
        author(&value).unwrap(),
        json!({"kind": "user", "id": Uuid::nil(), "email": email})
    );
}

#[test]
fn user_author_rejects_noncanonical_or_oversized_text() {
    for email in [
        String::new(),
        " ".into(),
        " owner@example.test".into(),
        "owner@example.test ".into(),
        "owner\n@example.test".into(),
        "owner\0@example.test".into(),
        "owner\u{7f}@example.test".into(),
        "\u{e9}".repeat(321),
    ] {
        assert_internal(author(&DeadlineActorSnapshot::User {
            id: UserId::from_uuid(Uuid::nil()),
            email,
        }));
    }
}

#[test]
fn technical_author_is_a_service_without_a_fabricated_user() {
    let value = DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    };
    assert_eq!(
        author(&value).unwrap(),
        json!({"kind": "technical", "service": "deadline_reevaluator", "policy_version": 1})
    );
}

#[test]
fn technical_author_rejects_unsupported_policy_versions() {
    for policy_version in [0, 2, u16::MAX] {
        assert_internal(author(&DeadlineActorSnapshot::Technical {
            service: TechnicalService::DeadlineReevaluator,
            policy_version,
        }));
    }
}

#[test]
fn legacy_receipt_has_only_its_version_tag() {
    assert_eq!(
        version(&DeadlineReceiptVersion::Legacy, case()).unwrap(),
        json!({"kind": "v1"})
    );
}

#[test]
fn human_registration_projects_explicit_null_predecessor_and_cause() {
    let value = DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
        observations_digest: digest(0),
        predecessor: None,
        cause: None,
    });
    assert_eq!(
        version(&value, case()).unwrap(),
        json!({
            "kind": "v2", "observations_digest": "00".repeat(32),
            "predecessor": null, "cause": null
        })
    );
}

#[test]
fn human_successor_preserves_both_predecessor_commitments() {
    assert_eq!(
        version(&tracked(None), case()).unwrap(),
        expected_tracked(Value::Null)
    );
}

#[test]
fn legacy_bootstrap_preserves_its_zero_job_uuid_and_policy() {
    let value = tracked(Some(TechnicalCause::LegacyBootstrap {
        job_id: Uuid::nil(),
        policy_version: 1,
    }));
    assert_eq!(
        version(&value, case()).unwrap(),
        expected_tracked(json!({
            "kind": "legacy_bootstrap", "job_id": Uuid::nil(), "policy_version": 1
        }))
    );
}

#[test]
fn legacy_bootstrap_rejects_unsupported_policy_versions() {
    for policy_version in [0, 2, u16::MAX] {
        let value = tracked(Some(TechnicalCause::LegacyBootstrap {
            job_id: Uuid::nil(),
            policy_version,
        }));
        assert_internal(version(&value, case()));
    }
}

#[test]
fn event_families_preserve_scope_and_large_decimal_sequences() {
    use DependencyFamily::*;
    for (family, name, scoped_case, hearing) in [
        (Resolution, "resolution", Some(case()), None),
        (Notification, "notification", Some(case()), None),
        (
            HearingResult,
            "hearing_result",
            Some(case()),
            Some(Uuid::nil()),
        ),
        (Calendar, "calendar", None, None),
        (Profile, "profile", None, None),
        (Profile, "profile", Some(case()), None),
    ] {
        let value = event_version(event(family, scoped_case, hearing));
        assert_eq!(
            version(&value, case()).unwrap(),
            expected_tracked(json!({
                "kind": "source_event", "job_id": Uuid::nil(),
                "event": {
                    "sequence": "9007199254740993", "family": name,
                    "source_id": Uuid::from_u128(2), "revision": 7,
                    "case_id": scoped_case, "hearing_id": hearing,
                    "operation_id": Uuid::from_u128(3)
                }
            }))
        );
    }
}

#[test]
fn event_preserves_zero_source_operation_case_and_hearing_ids() {
    let zero_case = CaseId::from_uuid(Uuid::nil());
    let value = event_version(SourceEventReference {
        sequence: 1,
        family: DependencyFamily::HearingResult,
        source_id: Uuid::nil(),
        revision: 1,
        case_id: Some(zero_case),
        hearing_id: Some(Uuid::nil()),
        operation_id: Uuid::nil(),
    });
    assert_eq!(
        version(&value, zero_case).unwrap(),
        expected_tracked(json!({
            "kind": "source_event", "job_id": Uuid::nil(),
            "event": {
                "sequence": "1", "family": "hearing_result",
                "source_id": Uuid::nil(), "revision": 1,
                "case_id": Uuid::nil(), "hearing_id": Uuid::nil(),
                "operation_id": Uuid::nil()
            }
        }))
    );
}

#[test]
fn event_accepts_the_largest_sequence_and_revision_without_rounding() {
    let mut value = event(DependencyFamily::Calendar, None, None);
    value.sequence = i64::MAX as u64;
    value.revision = u32::MAX;
    let projected = version(&event_version(value), case()).unwrap();
    assert_eq!(
        projected["cause"]["event"]["sequence"],
        "9223372036854775807"
    );
    assert_eq!(projected["cause"]["event"]["revision"], json!(u32::MAX));
}

#[test]
fn event_rejects_zero_or_overflowing_sequence_and_zero_revision() {
    for sequence in [0, i64::MAX as u64 + 1, u64::MAX] {
        let mut value = event(DependencyFamily::Calendar, None, None);
        value.sequence = sequence;
        assert_internal(version(&event_version(value), case()));
    }
    let mut value = event(DependencyFamily::Calendar, None, None);
    value.revision = 0;
    assert_internal(version(&event_version(value), case()));
}

#[test]
fn event_rejects_scope_and_hearing_combinations_for_every_family() {
    use DependencyFamily::*;
    let other = Some(CaseId::from_uuid(Uuid::from_u128(99)));
    let own = Some(case());
    let hearing = Some(Uuid::nil());
    for (family, scoped_case, hearing_id) in [
        (Resolution, None, None),
        (Resolution, other, None),
        (Resolution, own, hearing),
        (Notification, None, None),
        (Notification, other, None),
        (Notification, own, hearing),
        (HearingResult, None, None),
        (HearingResult, own, None),
        (HearingResult, None, hearing),
        (HearingResult, other, hearing),
        (Calendar, own, None),
        (Calendar, None, hearing),
        (Profile, other, None),
        (Profile, None, hearing),
        (Profile, own, hearing),
    ] {
        let value = event_version(event(family, scoped_case, hearing_id));
        assert_internal(version(&value, case()));
    }
}
