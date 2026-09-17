mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_sql_support;
use application::hearings::*;
use domain::identity::Role;
use hearing_database_support::*;
use hearing_sql_support::{insert, replacement, row, source};
use serde_json::json;

#[test]
fn direct_sql_rejects_different_sources_and_cancellation_values() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let svc = service(&db, db.owner, Role::Owner);
    let first = persist(&svc, db.case, schedule());
    let original = source(&mut db, first.snapshot.id);
    let command = replacement(&first);
    for (key, value) in [
        (
            "scheduling_administration_digest",
            json!(format!("\\x{}", "00".repeat(32))),
        ),
        (
            "scheduling_stage_digest",
            json!(format!("\\x{}", "00".repeat(32))),
        ),
        ("recorded_by_email", json!("forged@example.test")),
    ] {
        let mut attempted = row(&db, &original, &command, &first.snapshot.values);
        attempted[key] = value;
        let error = insert(&mut db.runtime(), &db.schema, &attempted).expect_err(key);
        assert!(
            matches!(error.code().map(|c| c.code()), Some("23514" | "42501")),
            "{error:?}"
        );
    }
    let cancel = HearingCommand {
        operation_id: HearingOperationId::new(),
        hearing_id: first.snapshot.id,
        change: HearingChange::Cancel {
            expected_revision: first.snapshot.revision,
            reason: HearingNote::new("Cancelled").unwrap(),
        },
    };
    let attempted = row(
        &db,
        &original,
        &cancel,
        &values("2026-09-18T09:00:00-06:00"),
    );
    assert_eq!(
        insert(&mut db.runtime(), &db.schema, &attempted)
            .unwrap_err()
            .code()
            .unwrap()
            .code(),
        "23514"
    );
}
#[test]
fn direct_sql_requires_contiguous_history_authorized_actor_and_read_committed() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let first = persist(&service(&db, db.owner, Role::Owner), db.case, schedule());
    let original = source(&mut db, first.snapshot.id);
    let mut command = replacement(&first);
    if let HearingChange::Replace {
        expected_revision, ..
    } = &mut command.change
    {
        *expected_revision = HearingRevision::new(3).unwrap();
    }
    let attempted = row(&db, &original, &command, &first.snapshot.values);
    assert_eq!(
        insert(&mut db.runtime(), &db.schema, &attempted)
            .unwrap_err()
            .code()
            .unwrap()
            .code(),
        "23514"
    );
    let attempted = row(&db, &original, &replacement(&first), &first.snapshot.values);
    let mut runtime = db.runtime();
    runtime
        .batch_execute("BEGIN ISOLATION LEVEL REPEATABLE READ")
        .unwrap();
    assert_eq!(
        insert(&mut runtime, &db.schema, &attempted)
            .unwrap_err()
            .code()
            .unwrap()
            .code(),
        "23514"
    );
    runtime.batch_execute("ROLLBACK").unwrap();
    db.admin
        .execute(
            "UPDATE users SET active=FALSE WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    assert_eq!(
        insert(&mut runtime, &db.schema, &attempted)
            .unwrap_err()
            .code()
            .unwrap()
            .code(),
        "42501"
    );
}
#[test]
fn startup_rejects_historical_source_corruption_and_disabled_sequence_guards() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    persist(&service(&db, db.owner, Role::Owner), db.case, schedule());
    db.admin.batch_execute("ALTER TABLE case_hearing_revisions DISABLE TRIGGER USER;
        UPDATE case_hearing_revisions SET scheduling_administration_digest=decode(repeat('00',32),'hex'),recorded_administration_digest=decode(repeat('00',32),'hex');
        ALTER TABLE case_hearing_revisions ENABLE TRIGGER USER").unwrap();
    assert!(infrastructure::PostgresHearingStore::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher),
        std::sync::Arc::new(case_stage_database_support::FixedClock(db.at))
    )
    .is_err());
}

#[test]
fn sql_cannot_record_a_replacement_under_later_administration_with_older_scheduling_context() {
    use application::cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation};
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let first = persist(&service(&db, db.owner, Role::Owner), db.case, schedule());
    let original = source(&mut db, first.snapshot.id);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let reopened = db
        .store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(2),
            CaseAdministrativeStatus::Active,
            db.at,
        )
        .unwrap();
    let admin = reopened.administration.snapshot().unwrap();
    let mut attempted = row(&db, &original, &replacement(&first), &first.snapshot.values);
    attempted["recorded_administration_revision"] = json!(admin.revision.get());
    attempted["recorded_administration_digest"] =
        json!(format!("\\x{}", admin.values_digest.to_hex()));
    assert_eq!(
        insert(&mut db.runtime(), &db.schema, &attempted)
            .unwrap_err()
            .code()
            .unwrap()
            .code(),
        "23514"
    );
    let cancelled = persist(
        &service(&db, db.owner, Role::Owner),
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: first.snapshot.id,
            change: HearingChange::Cancel {
                expected_revision: first.snapshot.revision,
                reason: HearingNote::new("Cancelled after administrative changes").unwrap(),
            },
        },
    );
    assert_eq!(cancelled.snapshot.recorded_administration_revision.get(), 3);
    assert_eq!(
        cancelled.snapshot.scheduling_context,
        first.snapshot.scheduling_context
    );
}
