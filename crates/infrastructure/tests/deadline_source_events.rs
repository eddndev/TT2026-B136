mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_source_event_support;
mod hearing_database_support;
mod hearing_result_database_support;
mod judicial_calendar_database_support;
mod procedural_fact_backend_support;

use application::cases::CaseRepository;
use deadline_source_event_support::*;

#[test]
fn all_four_source_families_emit_exact_events_on_creation_correction_and_withdrawal() {
    for kind in KINDS {
        let Some(mut db) = Fixture::new() else { return };
        let first = record(&mut db, kind);
        assert_events_match_sources(&mut db);
        let second = change(&db, &first, false);
        assert_events_match_sources(&mut db);
        change(&db, &second, true);
        assert_events_match_sources(&mut db);
        let rows = db.admin.query(
            "SELECT revision FROM deadline_source_events WHERE source_kind=$1 ORDER BY sequence",
            &[&kind],
        ).unwrap();
        assert_eq!(
            rows.iter().map(|r| r.get::<_, i64>(0)).collect::<Vec<_>>(),
            [1, 2, 3]
        );
    }
}

#[test]
fn audit_failure_rolls_back_the_source_revision_and_event_for_each_family() {
    for kind in KINDS {
        let Some(mut db) = Fixture::new() else { return };
        let first = record(&mut db, kind);
        let commit = prepared_correction(&db, &first);
        let before = snapshot(&mut db);
        reject_audit(&mut db);
        assert!(matches!(
            commit(),
            Err(application::ApplicationError::Port(_))
        ));
        assert_eq!(snapshot(&mut db), before);
        allow_audit(&mut db);
        change(&db, &first, false);
        assert_events_match_sources(&mut db);
    }
}

#[test]
fn direct_revision_inserts_emit_events_and_transaction_rollback_restores_every_row() {
    for kind in KINDS {
        let Some(mut db) = Fixture::new() else { return };
        record(&mut db, kind);
        let expected = event(&mut db, kind);
        let before = snapshot(&mut db);
        remove_event_temporarily(&mut db, &expected);
        replay_revision(&mut db, &expected);
        assert_events_match_sources(&mut db);
        db.admin.batch_execute("ROLLBACK").unwrap();
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn duplicate_source_revision_events_are_rejected_without_modifying_the_original() {
    for kind in KINDS {
        let Some(mut db) = Fixture::new() else { return };
        record(&mut db, kind);
        let expected = event(&mut db, kind);
        let before = event_rows(&mut db);
        let error = insert(&mut db, &expected).unwrap_err();
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::UNIQUE_VIOLATION)
        );
        assert_eq!(event_rows(&mut db), before);
    }
}

#[test]
fn stored_events_are_immutable_even_for_an_administrative_connection() {
    let Some(mut db) = Fixture::new() else { return };
    record(&mut db, "resolution");
    let before = snapshot(&mut db);
    for sql in [
        "UPDATE deadline_source_events SET operation_id=operation_id",
        "DELETE FROM deadline_source_events",
        "TRUNCATE deadline_source_events",
    ] {
        let error = db.admin.batch_execute(sql).unwrap_err();
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn runtime_has_only_insert_and_select_and_cannot_change_the_sequence_or_triggers() {
    let Some(mut db) = Fixture::new() else { return };
    record(&mut db, "resolution");
    let mut runtime = db.runtime();
    assert_eq!(
        runtime
            .query_one("SELECT count(*) FROM deadline_source_events", &[])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    for sql in [
        "UPDATE deadline_source_events SET operation_id=operation_id",
        "DELETE FROM deadline_source_events",
        "TRUNCATE deadline_source_events",
        "ALTER TABLE deadline_source_events DISABLE TRIGGER USER",
        "SELECT setval(pg_get_serial_sequence('deadline_source_events','sequence'),1)",
    ] {
        let error = runtime.batch_execute(sql).unwrap_err();
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::INSUFFICIENT_PRIVILEGE)
        );
    }
}

#[test]
fn events_require_the_exact_existing_revision_operation_and_typed_membership() {
    for kind in KINDS {
        let Some(mut db) = Fixture::new() else { return };
        record(&mut db, kind);
        let expected = event(&mut db, kind);
        let other_case = uuid::Uuid::new_v4();
        db.store()
            .create_basic(
                db.owner,
                domain::cases::CaseId::from_uuid(other_case),
                domain::cases::CaseMetadata::new("Other case", "OTHER").unwrap(),
                db.at,
            )
            .unwrap();
        let other_hearing = if kind == "hearing_result" {
            Some(
                hearing_database_support::persist(
                    &hearing_database_support::service(
                        &db,
                        db.owner,
                        domain::identity::Role::Owner,
                    ),
                    db.case,
                    hearing_database_support::schedule(),
                )
                .snapshot
                .id
                .as_uuid(),
            )
        } else {
            None
        };
        remove_event_temporarily(&mut db, &expected);
        let mut wrong = expected.clone();
        wrong.operation = uuid::Uuid::new_v4();
        rejects_event(&mut db, &wrong);
        wrong = expected.clone();
        wrong.revision += 1;
        rejects_event(&mut db, &wrong);
        wrong.revision = 0;
        rejects_event(&mut db, &wrong);
        wrong = expected.clone();
        wrong.id = uuid::Uuid::new_v4();
        rejects_event(&mut db, &wrong);
        wrong = expected.clone();
        wrong.case = Some(other_case);
        rejects_event(&mut db, &wrong);
        wrong = expected.clone();
        wrong.case = if kind == "calendar" {
            Some(db.case.as_uuid())
        } else {
            None
        };
        rejects_event(&mut db, &wrong);
        wrong = expected.clone();
        wrong.hearing = if kind == "hearing_result" {
            None
        } else {
            Some(uuid::Uuid::new_v4())
        };
        rejects_event(&mut db, &wrong);
        if let Some(other) = other_hearing {
            wrong = expected.clone();
            wrong.hearing = Some(other);
            rejects_event(&mut db, &wrong);
        }
        wrong = expected.clone();
        wrong.kind = "unsupported".into();
        rejects_event(&mut db, &wrong);
        assert_eq!(insert(&mut db, &expected).unwrap(), 1);
        db.admin.batch_execute("ROLLBACK").unwrap();
        assert_events_match_sources(&mut db);
    }
}

#[test]
fn equal_uuid_bytes_in_different_fact_families_produce_distinct_events() {
    use application::procedural_facts::*;
    use procedural_fact_backend_support as facts;
    let Some(mut db) = Fixture::new() else { return };
    let service = facts::service(&db, db.owner, domain::identity::Role::Owner);
    let resolution = facts::persist(&service, db.case, facts::record());
    let parent = facts::resolution_ref(&resolution);
    let notification = NotificationCommand::new(
        FactOperationId::new(),
        NotificationId::from_uuid(parent.id.as_uuid()),
        parent.id,
        FactChange::record(facts::notification_values(parent, "Declared notification")),
    )
    .unwrap();
    facts::persist(
        &service,
        db.case,
        ProceduralFactCommand::Notification(notification),
    );
    let first = event(&mut db, "resolution");
    let second = event(&mut db, "notification");
    assert_eq!(first.id, second.id);
    assert_eq!(first.revision, second.revision);
    assert_ne!(first.operation, second.operation);
    assert_events_match_sources(&mut db);
}

#[test]
fn callers_cannot_assign_the_ordering_sequence_explicitly() {
    let Some(mut db) = Fixture::new() else { return };
    record(&mut db, "resolution");
    let expected = event(&mut db, "resolution");
    remove_event_temporarily(&mut db, &expected);
    db.admin
        .batch_execute("SAVEPOINT explicit_sequence")
        .unwrap();
    let error = db.admin.execute("INSERT INTO deadline_source_events(sequence,source_kind,source_id,revision,case_id,operation_id) VALUES(999,'resolution',$1,$2,$3,$4)",
        &[&expected.id,&expected.revision,&expected.case,&expected.operation]).unwrap_err();
    assert_eq!(
        error.code(),
        Some(&postgres::error::SqlState::CHECK_VIOLATION)
    );
    db.admin
        .batch_execute("ROLLBACK TO SAVEPOINT explicit_sequence; ROLLBACK")
        .unwrap();
    assert_events_match_sources(&mut db);
}
