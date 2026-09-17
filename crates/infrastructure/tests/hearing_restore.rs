mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_sql_support;

use application::{case_stages::*, cases::*, hearings::*};
use domain::identity::Role;
use hearing_database_support::{complete, persist, schedule, service, store, Fixture};
use hearing_sql_support::{insert, replacement, row, source};

#[test]
fn full_dump_restore_preserves_exact_receipts_sources_and_inactive_historical_author() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let initial_case = db.case;
    let svc = service(&db, db.owner, Role::Owner);
    let first = persist(&svc, db.case, schedule());
    let cancelled = persist(
        &svc,
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: first.snapshot.id,
            change: HearingChange::Cancel {
                expected_revision: first.snapshot.revision,
                reason: HearingNote::new("Cancelled").unwrap(),
            },
        },
    );
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    complete(&mut db);
    let document = case_stage_database_support::upload(&db, db.case, "conviction.pdf");
    let stages = case_stage_database_support::service(
        &db,
        db.owner,
        Role::Owner,
        case_stage_database_support::FormatCheck(None),
    );
    stages
        .transition(
            "session",
            db.case,
            CaseStageRevision::FIRST,
            StageTransition::to_intermediate(
                DeclaredStageTime::instant(db.at).unwrap(),
                case_stage_database_support::reference(&document),
                None,
            ),
        )
        .unwrap();
    stages
        .transition(
            "session",
            db.case,
            CaseStageRevision::new(2).unwrap(),
            StageTransition::to_trial(
                DeclaredStageTime::instant(db.at).unwrap(),
                case_stage_database_support::reference(&document),
                DeclaredStageTime::instant(db.at).unwrap(),
                StageCourt::new("Trial court").unwrap(),
                Some(StageReceiptReference::new("Receipt A").unwrap()),
                Some(case_stage_database_support::reference(&document)),
                None,
            )
            .unwrap(),
        )
        .unwrap();
    let values = HearingValues::new(HearingValuesInput {
        kind: HearingKind::Sentencing,
        scheduled_at: first.snapshot.values.scheduled_at(),
        modality: HearingModality::Videoconference,
        venue: HearingVenue::new("Court B").unwrap(),
        note: None,
        participants: vec![],
        conviction_basis: Some(HearingConvictionBasis::new(
            HearingNote::new("Declared conviction").unwrap(),
            HearingSupportRef::new(
                application::documents::DocumentVersionRef {
                    id: document.id,
                    version: document.version,
                },
                document.digest,
            ),
        )),
    })
    .unwrap();
    let sentencing = persist(
        &svc,
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: HearingId::new(),
            change: HearingChange::Schedule {
                context: HearingContextExpectation {
                    case_revision: CaseRevision::FIRST,
                    stage_revision: CaseStageRevision::new(3).unwrap(),
                },
                values,
            },
        },
    );
    let reader = db.user("owner", false);
    db.admin
        .execute(
            "UPDATE users SET active=FALSE,email='former@example.test' WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    db.migrate();
    assert_eq!(snapshot(&mut db), before);
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("hearings.dump");
    run(std::process::Command::new("pg_dump")
        .args([
            "--dbname",
            &db.admin_url,
            "--schema",
            &db.schema,
            "--format=custom",
            "--file",
        ])
        .arg(&dump));
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    run(std::process::Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", &db.admin_url])
        .arg(&dump));
    assert_eq!(snapshot(&mut db), before);
    let restored = store(&db);
    assert_eq!(
        restored
            .get(
                reader,
                initial_case,
                first.snapshot.id,
                Some(first.snapshot.revision),
                db.at
            )
            .unwrap(),
        first
    );
    assert_eq!(
        restored
            .get(reader, initial_case, first.snapshot.id, None, db.at)
            .unwrap(),
        cancelled
    );
    assert_eq!(
        restored
            .get(reader, db.case, sentencing.snapshot.id, None, db.at)
            .unwrap(),
        sentencing
    );
    let result = persist(
        &service(&db, reader, Role::Owner),
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: sentencing.snapshot.id,
            change: HearingChange::Cancel {
                expected_revision: sentencing.snapshot.revision,
                reason: HearingNote::new("New recorded cancellation").unwrap(),
            },
        },
    );
    assert_eq!(result.snapshot.revision.get(), 2);
    assert_eq!(result.support, sentencing.support);
}
#[test]
fn canonical_checks_and_hearing_guard_work_with_empty_restore_search_path() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let first = persist(&service(&db, db.owner, Role::Owner), db.case, schedule());
    let original = source(&mut db, first.snapshot.id);
    let command = replacement(&first);
    let next = row(&db, &original, &command, &first.snapshot.values);
    let mut runtime = db.runtime();
    runtime.batch_execute("SET search_path=''").unwrap();
    insert(&mut runtime, &db.schema, &next).unwrap();
    assert_eq!(
        store(&db)
            .get(db.owner, db.case, first.snapshot.id, None, db.at)
            .unwrap()
            .snapshot
            .revision
            .get(),
        2
    );
}
fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM case_hearings h),'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY hearing_id,revision) FROM case_hearing_revisions r),'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))",&[]).unwrap().get(0)
}
fn run(command: &mut std::process::Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
