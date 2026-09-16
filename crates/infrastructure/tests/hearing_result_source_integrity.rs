mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_result_database_support;
mod hearing_result_sql_support;
use application::{
    cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation},
    hearing_results::*,
};
use domain::identity::Role;
use hearing_result_database_support::*;
use std::sync::Arc;

fn reopen(db: &Fixture, revision: u32) {
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(revision),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(revision + 1),
            CaseAdministrativeStatus::Active,
            db.at,
        )
        .unwrap();
}
fn forge_old_administration(db: &mut Fixture, id: HearingResultId, revision: u32) {
    db.admin
        .batch_execute("ALTER TABLE case_hearing_result_revisions DISABLE TRIGGER USER")
        .unwrap();
    db.admin.execute("UPDATE case_hearing_result_revisions SET recorded_administration_revision=1,
        recorded_administration_digest=(SELECT values_digest FROM case_administration_revisions WHERE case_id=$1 AND revision=1)
        WHERE result_id=$2 AND revision=$3", &[&db.case.as_uuid(), &id.as_uuid(), &i64::from(revision)]).unwrap();
    db.admin
        .batch_execute("ALTER TABLE case_hearing_result_revisions ENABLE TRIGGER USER")
        .unwrap();
}
fn rejects_open(db: &Fixture) {
    assert!(
        infrastructure::PostgresHearingResultStore::open(
            &db.runtime_url,
            Arc::new(infrastructure::RingSha256Hasher),
            Arc::new(case_stage_database_support::FixedClock(db.at))
        )
        .is_err(),
        "startup accepted historical administration preceding an exact source"
    );
}
#[test]
fn result_capture_cannot_precede_the_exact_cancelled_anchor_administration() {
    let Some(mut db) = Fixture::new() else { return };
    hearing_database_support::complete(&mut db);
    let hearings = hearing_database_support::service(&db, db.owner, Role::Owner);
    let scheduled =
        hearing_database_support::persist(&hearings, db.case, hearing_database_support::schedule());
    reopen(&db, 1);
    let cancelled = hearing_database_support::persist(
        &hearings,
        db.case,
        application::hearings::HearingCommand {
            operation_id: domain::hearings::HearingOperationId::new(),
            hearing_id: scheduled.snapshot.id,
            change: application::hearings::HearingChange::Cancel {
                expected_revision: scheduled.snapshot.revision,
                reason: domain::hearings::HearingNote::new("Cancel appointment").unwrap(),
            },
        },
    );
    let mut command = record(scheduled.snapshot.id);
    if let HearingResultChange::Record {
        anchor_revision, ..
    } = &mut command.change
    {
        *anchor_revision = cancelled.snapshot.revision;
    }
    let first = persist(&service(&db, db.owner, Role::Owner), db.case, command);
    assert_eq!(first.snapshot.recorded_administration_revision.get(), 3);
    forge_old_administration(&mut db, first.snapshot.id, 1);
    rejects_open(&db);
}
#[test]
fn successor_capture_cannot_precede_its_exact_previous_revision_administration() {
    let Some(mut db) = Fixture::new() else { return };
    let first = hearing_result_sql_support::seed(&mut db);
    let svc = service(&db, db.owner, Role::Owner);
    reopen(&db, 1);
    let second = persist(
        &svc,
        db.case,
        hearing_result_sql_support::correction(&first),
    );
    reopen(&db, 3);
    let third = persist(
        &svc,
        db.case,
        hearing_result_sql_support::correction(&second),
    );
    assert_eq!(third.snapshot.recorded_administration_revision.get(), 5);
    forge_old_administration(&mut db, third.snapshot.id, 3);
    rejects_open(&db);
}

#[test]
fn continuation_capture_cannot_precede_its_exact_antecedent_administration() {
    let Some(mut db) = Fixture::new() else { return };
    let first = hearing_result_sql_support::seed(&mut db);
    let svc = service(&db, db.owner, Role::Owner);
    reopen(&db, 1);
    let second = persist(
        &svc,
        db.case,
        hearing_result_sql_support::correction(&first),
    );
    let mut command = record(first.snapshot.hearing_id);
    if let HearingResultChange::Record { continuation, .. } = &mut command.change {
        *continuation = Some(HearingResultContinuationRef::new(
            second.snapshot.id,
            second.snapshot.revision,
        ));
    }
    let continued = persist(&svc, db.case, command);
    forge_old_administration(&mut db, continued.snapshot.id, 1);
    rejects_open(&db);
}
