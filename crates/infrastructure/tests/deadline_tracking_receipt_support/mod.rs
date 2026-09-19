use crate::case_administration_support::Fixture;
use application::deadline_reevaluation::*;
use domain::{
    crypto::Sha256Digest,
    deadlines::{DeadlineId, DeadlineOperationId},
};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn register(db: &Fixture) -> TrackedSubmission {
    TrackedSubmission {
        case_id: db.case,
        deadline_id: DeadlineId::from_uuid(Uuid::from_u128(2)),
        operation_id: DeadlineOperationId::from_uuid(Uuid::from_u128(3)),
        action: TrackedAction::Register,
        expected_revision: 0,
        review_digest: Sha256Digest::from_array([0x11; 32]),
        observations_digest: Sha256Digest::from_array([0x22; 32]),
        predecessor: None,
        author: TrackedAuthor::User {
            id: db.owner,
            email: "a".into(),
        },
        reason: None,
        cause: None,
    }
}

pub fn correction(db: &Fixture) -> TrackedSubmission {
    TrackedSubmission {
        action: TrackedAction::Correct,
        expected_revision: 7,
        predecessor: Some(PredecessorReceipt {
            submission_digest: Sha256Digest::from_array([0x33; 32]),
            capture_digest: Sha256Digest::from_array([0x44; 32]),
        }),
        reason: Some("Changed".into()),
        ..register(db)
    }
}

pub fn technical(db: &Fixture) -> TrackedSubmission {
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
                case_id: Some(db.case),
                hearing_id: Some(Uuid::from_u128(9)),
                operation_id: Uuid::from_u128(10),
            },
        }),
        ..correction(db)
    }
}

pub fn assert_projection(db: &mut Fixture, value: &TrackedSubmission) -> Vec<u8> {
    let bytes = encode_tracked_submission(value).unwrap();
    assert_eq!(decode_tracked_submission(&bytes).unwrap(), *value);
    let projected: Value = db
        .admin
        .query_one("SELECT deadline_submission($1)", &[&bytes])
        .expect("SQL must accept a valid DLTX2 receipt")
        .get(0);
    assert_eq!(projected, expected(value));
    bytes
}

fn digest(value: Sha256Digest) -> String {
    value
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn expected(value: &TrackedSubmission) -> Value {
    let action = match value.action {
        TrackedAction::Register => "register",
        TrackedAction::Correct => "correct",
        TrackedAction::SetAttention => "set_attention",
        TrackedAction::Retire => "retire",
        TrackedAction::Reevaluate => "reevaluate",
    };
    let author = match &value.author {
        TrackedAuthor::User { id, email } => {
            json!({"kind":"user","id":id.to_string(),"email":email})
        }
        TrackedAuthor::Technical { policy_version, .. } => json!({
            "kind":"technical", "service":"deadline_reevaluator",
            "policy_version":policy_version
        }),
    };
    let predecessor = value.predecessor.map(|base| {
        json!({
            "submission_digest":digest(base.submission_digest),
            "capture_digest":digest(base.capture_digest)
        })
    });
    let cause = value.cause.map(|cause| match cause {
        TechnicalCause::LegacyBootstrap {
            job_id,
            policy_version,
        } => json!({
            "kind":"legacy_bootstrap","job_id":job_id.to_string(),
            "policy_version":policy_version
        }),
        TechnicalCause::SourceEvent { job_id, event } => {
            let family = match event.family {
                DependencyFamily::Resolution => "resolution",
                DependencyFamily::Notification => "notification",
                DependencyFamily::HearingResult => "hearing_result",
                DependencyFamily::Calendar => "calendar",
                DependencyFamily::Profile => "profile",
            };
            json!({"kind":"source_event","job_id":job_id.to_string(),"event":{
                "sequence":event.sequence,"family":family,
                "source_id":event.source_id.to_string(),"revision":event.revision,
                "case_id":event.case_id.map(|id|id.to_string()),
                "hearing_id":event.hearing_id.map(|id|id.to_string()),
                "operation_id":event.operation_id.to_string()
            }})
        }
    });
    json!({
        "version":2,"case_id":value.case_id.to_string(),
        "deadline_id":value.deadline_id.to_string(),
        "operation_id":value.operation_id.to_string(),"action":action,
        "expected_revision":value.expected_revision,
        "review_digest":digest(value.review_digest),
        "observations_digest":digest(value.observations_digest),
        "predecessor":predecessor,"author":author,"reason":value.reason,"cause":cause
    })
}
