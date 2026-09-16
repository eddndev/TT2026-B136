mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod typed_participant_service_support;
use application::{typed_participants::*, ApplicationError};
use std::sync::{Arc, Barrier};
use typed_participant_service_support::*;
#[test]
fn concurrent_new_identities_against_one_reviewed_directory_have_one_success() {
    let Some(mut db) = Fixture::new() else { return };
    let record = upload(&db, db.case, "support.pdf");
    let barrier = Arc::new(Barrier::new(2));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let barrier = barrier.clone();
        let workflow = service(
            &db,
            FormatCheck(Some(Box::new(move || {
                barrier.wait();
            }))),
        );
        let request = reviewed(
            workflow
                .review_participant("session", db.case, proposal(&record))
                .unwrap(),
        );
        let case = db.case;
        workers.push(std::thread::spawn(move || {
            workflow.submit_participant(
                "session",
                case,
                ParticipantSubmission {
                    prepared: request,
                    signature: None,
                },
            )
        }));
    }
    let results = workers
        .into_iter()
        .map(|w| w.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(ApplicationError::ParticipantIdentityReviewConflict)))
            .count(),
        1,
        "{results:?}"
    );
    let row=db.admin.query_one("SELECT (SELECT count(*) FROM case_subjects),(SELECT count(*) FROM case_participant_typed_revisions),(SELECT count(*) FROM audit_events WHERE action='participant.typed_created')",&[]).unwrap();
    assert_eq!(
        (
            row.get::<_, i64>(0),
            row.get::<_, i64>(1),
            row.get::<_, i64>(2)
        ),
        (1, 1, 1)
    );
}
#[test]
fn stale_role_base_and_archived_duplicate_role_require_explicit_reconciliation() {
    let Some(db) = Fixture::new() else { return };
    let record = upload(&db, db.case, "support.pdf");
    let workflow = service(&db, FormatCheck(None));
    let request = reviewed(
        workflow
            .review_participant("session", db.case, proposal(&record))
            .unwrap(),
    );
    let committed = workflow
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: request,
                signature: None,
            },
        )
        .unwrap();
    let ParticipantRevisionSnapshot::Typed(typed) = committed.revision else {
        panic!()
    };
    let value = ParticipantProposalRequest {
        subject: SubjectDraftSelection::Keep(typed.values.subject()),
        participant: ParticipantDraftTarget::Create,
        role: typed.values.role().clone(),
        certificate: None,
    };
    let duplicate = reviewed(
        workflow
            .review_participant("session", db.case, value)
            .unwrap(),
    );
    use application::participants::ParticipantStore;
    let manual = infrastructure::PostgresParticipantStore::open(
        &db.runtime_url,
        Arc::new(infrastructure::RingSha256Hasher),
    )
    .unwrap();
    manual
        .change_status(
            db.owner,
            db.case,
            typed.id,
            typed.revision,
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    assert!(matches!(
        workflow.submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: duplicate,
                signature: None
            }
        ),
        Err(ApplicationError::ParticipantRoleConflict)
    ));
}
