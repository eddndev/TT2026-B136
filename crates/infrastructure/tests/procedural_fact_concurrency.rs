mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;
mod procedural_fact_concurrency_support;
use application::{cases::*, procedural_facts::*, ApplicationError};
use domain::{
    cases::CaseMetadata,
    clock::{Clock, OffsetDateTime},
    identity::Role,
};
use procedural_fact_backend_support::*;
use procedural_fact_concurrency_support::*;
use std::sync::{
    atomic::{AtomicI64, Ordering},
    mpsc, Arc, Barrier, Mutex,
};
use std::time::Duration;

#[test]
fn competing_corrections_commit_one_exact_successor_and_one_audit_event() {
    for notification in [false, true] {
        let Some(mut db) = Fixture::new() else { return };
        let workflow = service(&db, db.owner, Role::Owner);
        let resolution = persist(&workflow, db.case, record());
        let original = if notification {
            persist(&workflow, db.case, notify(resolution_ref(&resolution)))
        } else {
            resolution
        };
        let commands = [correct(&original), correct(&original)];
        assert_ne!(commands[0].operation_id(), commands[1].operation_id());
        let before = counts(&mut db);
        let barrier = Arc::new(Barrier::new(2));
        let workers = commands
            .into_iter()
            .map(|command| {
                let draft = workflow
                    .prepare("session", db.case, command.clone())
                    .unwrap();
                let barrier = barrier.clone();
                let inner = before_commit(store(&db), move |_| {
                    barrier.wait();
                });
                let runner = service_with_store(&db, inner, db.owner, Role::Owner);
                let case = db.case;
                std::thread::spawn(move || {
                    runner.submit("session", case, command, draft.submission_digest)
                })
            })
            .collect::<Vec<_>>();
        let outcomes = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            outcomes.iter().filter(|result| result.is_ok()).count(),
            1,
            "{outcomes:?}"
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|result| matches!(
                    result,
                    Err(ApplicationError::ProceduralFact(
                        ProceduralFactError::RevisionConflict
                    ))
                ))
                .count(),
            1,
            "{outcomes:?}"
        );
        let winner = outcomes.into_iter().find_map(Result::ok).unwrap();
        assert_eq!(winner.snapshot.target(), original.snapshot.target());
        assert_eq!(winner.snapshot.metadata().revision.get(), 2);
        assert_eq!(winner.snapshot.metadata().receipt.expected_revision, 1);
        assert_eq!(counts(&mut db), (before.0, before.1 + 1, before.2 + 1));
        assert_eq!(
            workflow
                .get(
                    "session",
                    db.case,
                    original.snapshot.target(),
                    Some(FactRevision::initial())
                )
                .unwrap(),
            original
        );
        assert_eq!(
            workflow
                .get("session", db.case, winner.snapshot.target(), None)
                .unwrap(),
            winner
        );
    }
}

#[test]
fn commit_rechecks_membership_role_and_active_identity_after_service_reauthentication() {
    for (mutation, expected) in [
        (
            "DELETE FROM case_memberships WHERE user_id=$1",
            "membership",
        ),
        ("UPDATE users SET active=FALSE WHERE id=$1", "active"),
        ("UPDATE users SET role='paralegal' WHERE id=$1", "role"),
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let actor = db.user("litigator", true);
        let command = record();
        let draft = service(&db, actor, Role::Litigator)
            .prepare("session", db.case, command.clone())
            .unwrap();
        let (workflow, captured) = watched(&db, actor, Role::Litigator, move |client| {
            client.execute(mutation, &[&actor.as_uuid()]).unwrap();
        });
        let result = workflow.submit("session", db.case, command, draft.submission_digest);
        match expected {
            "membership" => assert!(
                matches!(result, Err(ApplicationError::CaseNotFound)),
                "{result:?}"
            ),
            "active" => assert!(
                matches!(result, Err(ApplicationError::InvalidSession)),
                "{result:?}"
            ),
            _ => assert!(
                matches!(result, Err(ApplicationError::PermissionDenied)),
                "{result:?}"
            ),
        }
        unchanged(&mut db, captured);
    }
}

#[test]
fn case_closed_immediately_before_commit_leaves_no_fact_or_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let command = record();
    let draft = service(&db, db.owner, Role::Owner)
        .prepare("session", db.case, command.clone())
        .unwrap();
    assert!(matches!(
        draft.observed_administration,
        CurrentCaseAdministration::Unrevised(_)
    ));
    let repository = db.store();
    let (actor, case, at) = (db.owner, db.case, db.at);
    let (workflow, captured) = watched(&db, actor, Role::Owner, move |_| {
        repository
            .change_administrative_status(
                actor,
                case,
                CaseRevisionExpectation::Unrevised,
                CaseAdministrativeStatus::Closed,
                at,
            )
            .unwrap();
    });
    let result = workflow.submit("session", case, command, draft.submission_digest);
    assert!(
        matches!(result, Err(ApplicationError::CaseClosed)),
        "{result:?}"
    );
    unchanged(&mut db, captured);
}

#[test]
fn active_administration_can_advance_from_unrevised_to_one_without_a_cas_conflict() {
    let Some(mut db) = Fixture::new() else { return };
    let command = record();
    let draft = service(&db, db.owner, Role::Owner)
        .prepare("session", db.case, command.clone())
        .unwrap();
    assert!(matches!(
        draft.observed_administration,
        CurrentCaseAdministration::Unrevised(_)
    ));
    let repository = db.store();
    let (actor, case, at) = (db.owner, db.case, db.at);
    let saved = Arc::new(Mutex::new(None));
    let capture = saved.clone();
    let before = counts(&mut db);
    let inner = before_commit(store(&db), move |prepared| {
        assert!(matches!(
            prepared.preparation().observed_administration,
            CurrentCaseAdministration::Unrevised(_)
        ));
        let changed = repository
            .replace_administration(
                actor,
                case,
                CaseRevisionExpectation::Unrevised,
                CaseEditableValues::new(
                    CaseMetadata::new("Later case title", "REF-OLD").unwrap(),
                    None,
                ),
                at,
            )
            .unwrap();
        *capture.lock().unwrap() = Some(changed.administration);
    });
    let result = service_with_store(&db, inner, actor, Role::Owner)
        .submit("session", case, command, draft.submission_digest)
        .unwrap();
    let CurrentCaseAdministration::Recorded(administration) =
        &result.snapshot.metadata().recorded_administration
    else {
        panic!("commit did not capture the new administration")
    };
    assert_eq!(administration.revision.get(), 1);
    assert_eq!(administration.values.metadata().title(), "Later case title");
    assert!(administration.values.profile().is_none());
    assert_eq!(
        result.snapshot.metadata().recorded_administration,
        saved.lock().unwrap().clone().unwrap()
    );
    assert_eq!(
        result.snapshot.metadata().receipt.submission_digest,
        draft.submission_digest
    );
    assert_eq!(counts(&mut db), (before.0 + 1, before.1 + 1, before.2 + 2));
}

struct AdjustableClock(AtomicI64);
impl Clock for AdjustableClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.0.load(Ordering::SeqCst)).unwrap()
    }
}

#[test]
fn capture_clock_is_observed_only_after_the_commit_obtains_the_audit_lock() {
    let Some(mut db) = Fixture::new() else { return };
    let command = record();
    let draft = service(&db, db.owner, Role::Owner)
        .prepare("session", db.case, command.clone())
        .unwrap();
    let clock = Arc::new(AdjustableClock(AtomicI64::new(db.at.unix_timestamp())));
    let later = db.at.unix_timestamp() + 600;
    let (ready_tx, ready_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let resume_rx = Mutex::new(resume_rx);
    let inner = before_commit(clocked_store(&db, clock.clone()), move |_| {
        ready_tx.send(()).unwrap();
        resume_rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    });
    let workflow = service_with_store(&db, inner, db.owner, Role::Owner);
    let case = db.case;
    let worker = std::thread::spawn(move || {
        workflow.submit("session", case, command, draft.submission_digest)
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    db.admin
        .batch_execute("SELECT pg_advisory_lock(280603412820)")
        .unwrap();
    resume_tx.send(()).unwrap();
    let waiting = wait_for_audit_lock(&mut db);
    clock.0.store(later, Ordering::SeqCst);
    db.admin
        .batch_execute("SELECT pg_advisory_unlock(280603412820)")
        .unwrap();
    assert!(
        waiting,
        "fact commit did not wait for the common audit lock"
    );
    let result = worker.join().unwrap().unwrap();
    assert_eq!(
        result.snapshot.metadata().recorded_at.unix_timestamp(),
        later
    );
    assert_eq!(result.snapshot.metadata().recorded_at.nanosecond(), 0);
    assert!(matches!(
        result.snapshot.metadata().recorded_administration,
        CurrentCaseAdministration::Unrevised(_)
    ));
}
