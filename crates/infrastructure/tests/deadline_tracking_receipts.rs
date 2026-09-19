mod case_administration_support;
mod deadline_schema_support;
mod deadline_tracking_receipt_support;

use application::deadline_reevaluation::*;
use case_administration_support::Fixture;
use deadline_tracking_receipt_support::*;
use domain::{
    cases::CaseId,
    deadlines::{DeadlineId, DeadlineOperationId},
    identity::UserId,
};
use serde_json::{json, Value};
use uuid::Uuid;

#[test]
fn sql_preserves_the_existing_v1_json_without_version_or_author_fields() {
    let Some(mut db) = Fixture::new() else { return };
    for (action, label, expected, reason) in [
        (0, "register", 0, None),
        (1, "correct", 1, Some("Corrected")),
        (2, "set_attention", 1, Some("Attended")),
        (3, "retire", u32::MAX - 1, Some("Retired")),
    ] {
        let bytes = deadline_schema_support::receipt(
            db.owner.as_uuid(),
            db.case.as_uuid(),
            Uuid::nil(),
            Uuid::nil(),
            action,
            expected,
            reason,
        );
        let projected: Value = db
            .admin
            .query_one("SELECT deadline_submission($1)", &[&bytes])
            .unwrap()
            .get(0);
        assert_eq!(
            projected,
            json!({
                "actor_id":db.owner.to_string(),"case_id":db.case.to_string(),
                "deadline_id":Uuid::nil().to_string(),"operation_id":Uuid::nil().to_string(),
                "action":label,"expected_revision":expected,"review_digest":"17".repeat(32),
                "reason":reason
            })
        );
    }
}

#[test]
fn sql_projects_all_manual_actions_and_both_predecessor_commitments() {
    let Some(mut db) = Fixture::new() else { return };
    let value = register(&db);
    assert_eq!(assert_projection(&mut db, &value).len(), 151);
    for action in [
        TrackedAction::Correct,
        TrackedAction::SetAttention,
        TrackedAction::Retire,
    ] {
        let value = TrackedSubmission {
            action,
            ..correction(&db)
        };
        assert_eq!(assert_projection(&mut db, &value).len(), 230);
    }
}

#[test]
fn sql_projects_each_source_event_family_and_both_profile_scopes() {
    let Some(mut db) = Fixture::new() else { return };
    for (family, scoped, hearing) in [
        (DependencyFamily::Resolution, true, false),
        (DependencyFamily::Notification, true, false),
        (DependencyFamily::HearingResult, true, true),
        (DependencyFamily::Calendar, false, false),
        (DependencyFamily::Profile, false, false),
        (DependencyFamily::Profile, true, false),
    ] {
        let mut value = technical(&db);
        let Some(TechnicalCause::SourceEvent { event, .. }) = &mut value.cause else {
            panic!("source event fixture expected");
        };
        event.family = family;
        event.case_id = scoped.then_some(db.case);
        event.hearing_id = hearing.then_some(Uuid::from_u128(9));
        assert_projection(&mut db, &value);
    }
}

#[test]
fn sql_projects_legacy_bootstrap_as_a_technical_cause_without_a_source_event() {
    let Some(mut db) = Fixture::new() else { return };
    let value = TrackedSubmission {
        cause: Some(TechnicalCause::LegacyBootstrap {
            job_id: Uuid::nil(),
            policy_version: 1,
        }),
        ..technical(&db)
    };
    assert_eq!(assert_projection(&mut db, &value).len(), 226);
}

#[test]
fn sql_preserves_maximum_utf8_scalar_lengths_and_the_5502_byte_bound() {
    let Some(mut db) = Fixture::new() else { return };
    let mut value = correction(&db);
    value.author = TrackedAuthor::User {
        id: UserId::from_uuid(Uuid::nil()),
        email: "\u{1f4c4}".repeat(320),
    };
    value.reason = Some("\u{1f4c4}".repeat(1000));
    assert_eq!(assert_projection(&mut db, &value).len(), 5502);
    value.reason = Some("Same\ntext".into());
    assert_projection(&mut db, &value);
}

#[test]
fn sql_preserves_nil_identifiers_and_unsigned_revision_limits() {
    let Some(mut db) = Fixture::new() else { return };
    let mut value = technical(&db);
    value.case_id = CaseId::from_uuid(Uuid::nil());
    value.deadline_id = DeadlineId::from_uuid(Uuid::nil());
    value.operation_id = DeadlineOperationId::from_uuid(Uuid::nil());
    value.expected_revision = u32::MAX - 1;
    let Some(TechnicalCause::SourceEvent { job_id, event }) = &mut value.cause else {
        panic!("source event fixture expected");
    };
    *job_id = Uuid::nil();
    event.sequence = i64::MAX as u64;
    event.source_id = Uuid::nil();
    event.revision = u32::MAX;
    event.case_id = Some(value.case_id);
    event.hearing_id = Some(Uuid::nil());
    event.operation_id = Uuid::nil();
    assert_projection(&mut db, &value);
    let value = TrackedSubmission {
        case_id: CaseId::from_uuid(Uuid::nil()),
        deadline_id: DeadlineId::from_uuid(Uuid::nil()),
        operation_id: DeadlineOperationId::from_uuid(Uuid::nil()),
        author: TrackedAuthor::User {
            id: UserId::from_uuid(Uuid::nil()),
            email: "a".into(),
        },
        ..register(&db)
    };
    assert_projection(&mut db, &value);
}
