mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_result_database_support;
mod hearing_result_revalidation_support;
mod hearing_result_sql_support;

use application::{hearing_results::*, ApplicationError};
use domain::identity::Role;
use hearing_result_database_support::*;
use hearing_result_revalidation_support::*;
use hearing_result_sql_support::correction;
use std::sync::{Arc, Barrier};

#[test]
fn simultaneous_correction_and_withdrawal_have_exactly_one_audited_successor() {
    let Some(mut db) = Fixture::new() else { return };
    let anchor = appointment(&mut db);
    let initial = persist(
        &service(&db, db.owner, Role::Owner),
        db.case,
        record(anchor.snapshot.id),
    );
    let before = counts(&mut db);
    let barrier = Arc::new(Barrier::new(2));
    let commands = [
        correction(&initial),
        HearingResultCommand {
            operation_id: HearingResultOperationId::new(),
            hearing_id: initial.snapshot.hearing_id,
            result_id: initial.snapshot.id,
            change: HearingResultChange::Withdraw {
                expected_revision: initial.snapshot.revision,
                reason: HearingResultText::new("Cancellation").unwrap(),
            },
        },
    ];
    let workers = commands
        .into_iter()
        .map(|command| {
            let draft = service(&db, db.owner, Role::Owner)
                .prepare("session", db.case, command.clone())
                .unwrap();
            let barrier = barrier.clone();
            let workflow = hooked(&db, db.owner, Role::Owner, move || {
                barrier.wait();
            });
            let case = db.case;
            std::thread::spawn(move || {
                workflow.submit("session", case, command, draft.submission_digest)
            })
        })
        .collect::<Vec<_>>();
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
            .filter(|r| matches!(
                r,
                Err(ApplicationError::HearingResult(
                    HearingResultError::RevisionConflict
                ))
            ))
            .count(),
        1,
        "{results:?}"
    );
    assert_eq!(counts(&mut db), (before.0, before.1 + 1, before.2 + 1));
}

#[test]
fn simultaneous_roots_with_one_operation_id_cannot_both_commit() {
    let Some(mut db) = Fixture::new() else { return };
    let anchor = appointment(&mut db);
    let before = counts(&mut db);
    let barrier = Arc::new(Barrier::new(2));
    let operation = HearingResultOperationId::new();
    let workers = (0..2)
        .map(|_| {
            let mut command = record(anchor.snapshot.id);
            command.operation_id = operation;
            let draft = service(&db, db.owner, Role::Owner)
                .prepare("session", db.case, command.clone())
                .unwrap();
            let barrier = barrier.clone();
            let workflow = hooked(&db, db.owner, Role::Owner, move || {
                barrier.wait();
            });
            let case = db.case;
            std::thread::spawn(move || {
                workflow.submit("session", case, command, draft.submission_digest)
            })
        })
        .collect::<Vec<_>>();
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
            .filter(|r| matches!(
                r,
                Err(ApplicationError::HearingResult(
                    HearingResultError::OperationConflict
                ))
            ))
            .count(),
        1,
        "{results:?}"
    );
    assert_eq!(counts(&mut db), (before.0 + 1, before.1 + 1, before.2 + 1));
}

#[test]
fn waiting_read_rechecks_membership_under_read_committed() {
    let Some(mut db) = Fixture::new() else { return };
    let anchor = appointment(&mut db);
    let actor = db.user("litigator", true);
    let recorded = persist(
        &service(&db, db.owner, Role::Owner),
        db.case,
        record(anchor.snapshot.id),
    );
    db.admin
        .batch_execute(&format!(
            "ALTER ROLE {} SET default_transaction_isolation='repeatable read'",
            db.role
        ))
        .unwrap();
    let hearings = store(&db);
    let before = snapshot(&mut db.admin);
    db.admin
        .batch_execute("SELECT pg_advisory_lock(280603412820)")
        .unwrap();
    let (case, hearing, id, at) = (db.case, anchor.snapshot.id, recorded.snapshot.id, db.at);
    let worker = std::thread::spawn(move || hearings.get(actor, case, hearing, id, None, at));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let observed = loop {
        let waiting: bool = db.admin.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename=$1 AND wait_event='advisory')", &[&db.role]).unwrap().get(0);
        if waiting {
            break true;
        }
        if std::time::Instant::now() >= deadline {
            break false;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&case.as_uuid(), &actor.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("SELECT pg_advisory_unlock(280603412820)")
        .unwrap();
    assert!(
        observed,
        "hearing result read did not wait for the audit lock"
    );
    assert!(matches!(
        worker.join().unwrap(),
        Err(ApplicationError::CaseNotFound)
    ));
    assert_eq!(snapshot(&mut db.admin), before);
}
