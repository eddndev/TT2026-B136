mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_revalidation_support;

use application::{
    case_stages::*,
    documents::{CaseDocumentStore, DocumentRecord},
    hearings::*,
    ApplicationError,
};
use domain::{case_administration::CaseRevision, identity::Role};
use hearing_database_support::*;
use hearing_revalidation_support::*;
use infrastructure::PostgresCaseDocumentStore;

fn trial(db: &Fixture) -> DocumentRecord {
    use case_stage_database_support as stages;
    let record = stages::upload(db, db.case, "judgment.pdf");
    let workflow = stages::service(db, db.owner, Role::Owner, stages::FormatCheck(None));
    let instant = DeclaredStageTime::instant(db.at).unwrap();
    workflow
        .transition(
            "session",
            db.case,
            CaseStageRevision::FIRST,
            StageTransition::to_intermediate(instant, stages::reference(&record), None),
        )
        .unwrap();
    workflow
        .transition(
            "session",
            db.case,
            CaseStageRevision::new(2).unwrap(),
            StageTransition::to_trial(
                instant,
                stages::reference(&record),
                instant,
                StageCourt::new("Trial court").unwrap(),
                None,
                None,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    record
}
fn sentencing(record: &DocumentRecord) -> HearingCommand {
    let mut command = schedule();
    let HearingChange::Schedule { context, values } = &mut command.change else {
        unreachable!()
    };
    *context = HearingContextExpectation {
        case_revision: CaseRevision::FIRST,
        stage_revision: CaseStageRevision::new(3).unwrap(),
    };
    *values = HearingValues::new(HearingValuesInput {
        kind: HearingKind::Sentencing,
        scheduled_at: values.scheduled_at(),
        modality: values.modality(),
        venue: values.venue().clone(),
        note: None,
        participants: vec![],
        conviction_basis: Some(HearingConvictionBasis::new(
            HearingNote::new("Declared conviction for scheduling").unwrap(),
            HearingSupportRef::new(
                application::documents::DocumentVersionRef {
                    id: record.id,
                    version: record.version,
                },
                record.digest,
            ),
        )),
    })
    .unwrap();
    command
}

#[test]
fn seal_during_preparation_conflicts_and_later_versions_do_not_replace_exact_support() {
    for seal in [true, false] {
        let Some(mut db) = Fixture::new() else { return };
        complete(&mut db);
        let record = trial(&db);
        let command = sentencing(&record);
        let workflow = service(&db, db.owner, Role::Owner);
        let draft = workflow
            .prepare("session", db.case, command.clone())
            .unwrap();
        let documents = PostgresCaseDocumentStore::open(&db.runtime_url).unwrap();
        let processor = case_stage_database_support::processor();
        let modified = if seal {
            processor.seal(&record).unwrap()
        } else {
            processor
                .prepare_version(
                    record.id,
                    record.version.next().unwrap(),
                    "new-judgment.pdf",
                    b"new exact content",
                )
                .unwrap()
        };
        let (actor, case, at, version) = (db.owner, db.case, db.at, record.version);
        let (interleaved, captured) = watched(&db, actor, Role::Owner, move |_| {
            if seal {
                documents.seal(actor, case, modified.clone(), at).unwrap();
            } else {
                documents
                    .append(actor, case, version, modified.clone(), at)
                    .unwrap();
            }
        });
        let result = interleaved.submit("session", case, command, draft.submission_digest);
        if seal {
            assert!(
                matches!(
                    result,
                    Err(ApplicationError::Hearing(HearingError::SupportChanged))
                ),
                "{result:?}"
            );
            unchanged(&mut db, captured);
        } else {
            let result = result.unwrap();
            let support = result.support.as_ref().unwrap();
            assert_eq!(support.reference.version, record.version);
            assert_eq!(support.digest, record.digest);
            assert_eq!(support.name, record.name);
            assert_eq!(
                workflow
                    .get("session", case, result.snapshot.id, None)
                    .unwrap(),
                result
            );
        }
    }
}

#[test]
fn cancellation_copies_captured_support_without_reopening_or_revalidating_the_file() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let record = trial(&db);
    let workflow = service(&db, db.owner, Role::Owner);
    let initial = persist(&workflow, db.case, sentencing(&record));
    db.admin
        .batch_execute("ALTER TABLE documents DISABLE TRIGGER USER")
        .unwrap();
    db.admin.execute("UPDATE documents SET vault=set_byte(vault,octet_length(vault)-1,get_byte(vault,octet_length(vault)-1)#1) WHERE id=$1", &[&record.id.as_uuid()]).unwrap();
    db.admin
        .batch_execute("ALTER TABLE documents ENABLE TRIGGER USER")
        .unwrap();
    assert!(workflow
        .prepare("session", db.case, replacement(&initial))
        .is_err());
    let cancelled = persist(
        &workflow,
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: initial.snapshot.id,
            change: HearingChange::Cancel {
                expected_revision: initial.snapshot.revision,
                reason: HearingNote::new("Appointment cancelled").unwrap(),
            },
        },
    );
    assert_eq!(cancelled.support, initial.support);
    assert_eq!(
        cancelled.snapshot.values_digest,
        initial.snapshot.values_digest
    );
    assert_eq!(cancelled.snapshot.status, HearingStatus::Cancelled);
}

#[test]
fn read_audit_failure_does_not_expose_context_detail_history_or_lists() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let initial = persist(&workflow, db.case, schedule());
    db.admin.batch_execute("ALTER TABLE audit_events ADD CONSTRAINT reject_hearing_read CHECK(action NOT LIKE 'hearing.%read')").unwrap();
    let before = snapshot(&mut db.admin);
    let from = HearingTime::new(
        time::OffsetDateTime::parse(
            "2026-09-01T00:00:00Z",
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap(),
    )
    .unwrap();
    let until = HearingTime::new(
        time::OffsetDateTime::parse(
            "2026-10-01T00:00:00Z",
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap(),
    )
    .unwrap();
    for result in [
        workflow.context("session", db.case).map(|_| ()),
        workflow
            .get("session", db.case, initial.snapshot.id, None)
            .map(|_| ()),
        workflow
            .history(
                "session",
                db.case,
                initial.snapshot.id,
                HearingHistoryQuery::new(20, None).unwrap(),
            )
            .map(|_| ()),
        workflow
            .list(
                "session",
                db.case,
                HearingQuery::new(20, None, HearingStatusFilter::All).unwrap(),
            )
            .map(|_| ()),
        workflow
            .agenda(
                "session",
                HearingAgendaQuery::new(
                    20,
                    from.utc(),
                    until.utc(),
                    None,
                    HearingStatusFilter::All,
                )
                .unwrap(),
            )
            .map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(ApplicationError::Port(_))),
            "{result:?}"
        );
    }
    assert_eq!(snapshot(&mut db.admin), before);
}
