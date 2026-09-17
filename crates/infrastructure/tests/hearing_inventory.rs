mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_sql_support;
use application::{hearings::*, participants::*};
use domain::identity::Role;
use hearing_database_support::*;
use hearing_sql_support::{insert, row, source};
use std::sync::Arc;

#[test]
fn historical_reads_and_startup_reject_a_revision_that_selected_an_archived_snapshot() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let participants = infrastructure::PostgresParticipantStore::open(
        &db.runtime_url,
        Arc::new(infrastructure::RingSha256Hasher),
    )
    .unwrap();
    let person = participants
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            ParticipantValues::new("Witness", "Witness", None, None, DirectoryStatus::Active)
                .unwrap(),
            db.at,
        )
        .unwrap();
    let archived = participants
        .change_status(
            db.owner,
            db.case,
            person.id,
            person.revision,
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    let svc = service(&db, db.owner, Role::Owner);
    let first = persist(&svc, db.case, schedule());
    let original = source(&mut db, first.snapshot.id);
    let values = HearingValues::new(HearingValuesInput {
        kind: first.snapshot.values.kind(),
        scheduled_at: first.snapshot.values.scheduled_at(),
        modality: first.snapshot.values.modality(),
        venue: first.snapshot.values.venue().clone(),
        note: None,
        participants: vec![HearingParticipantRef::new(
            archived.id(),
            archived.revision_number(),
        )],
        conviction_basis: None,
    })
    .unwrap();
    let command = HearingCommand {
        operation_id: HearingOperationId::new(),
        hearing_id: first.snapshot.id,
        change: HearingChange::Replace {
            expected_revision: first.snapshot.revision,
            context: context(),
            values: values.clone(),
            reason: HearingNote::new("Injected archived selection").unwrap(),
        },
    };
    let forged = row(&db, &original, &command, &values);
    db.admin
        .batch_execute("ALTER TABLE case_hearing_revisions DISABLE TRIGGER hearing_sequence")
        .unwrap();
    insert(&mut db.admin, &db.schema, &forged).unwrap();
    db.admin
        .batch_execute("ALTER TABLE case_hearing_revisions ENABLE TRIGGER hearing_sequence")
        .unwrap();
    assert!(svc
        .get("session", db.case, first.snapshot.id, None)
        .is_err());
    assert!(infrastructure::PostgresHearingStore::open(
        &db.runtime_url,
        Arc::new(infrastructure::RingSha256Hasher),
        Arc::new(case_stage_database_support::FixedClock(db.at))
    )
    .is_err());
}
