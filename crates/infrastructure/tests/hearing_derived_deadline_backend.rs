use crate::{
    case_administration_support::Fixture,
    case_stage_database_support::{processor, FixedClock, FormatCheck, TestIdentity},
    hearing_derived_deadline_storage_support as support,
};
use application::hearing_derived_deadlines::*;
use infrastructure::{PostgresHearingDerivedDeadlineStore, RingSha256Hasher};
use std::sync::Arc;

fn service(db: &Fixture) -> HearingDerivedDeadlineService {
    HearingDerivedDeadlineService::new(
        Arc::new(
            PostgresHearingDerivedDeadlineStore::open(
                &db.runtime_url,
                Arc::new(RingSha256Hasher),
                Arc::new(FixedClock(db.at)),
            )
            .unwrap(),
        ),
        Arc::new(TestIdentity(application::identity::Principal {
            id: db.owner,
            email: "owner@example.test".into(),
            role: domain::identity::Role::Owner,
        })),
        processor(),
        Arc::new(FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

fn counts(db: &mut Fixture) -> (i64, i64, i64, i64, i64) {
    let row = db.admin.query_one("SELECT
        (SELECT count(*) FROM case_hearing_result_revisions),
        (SELECT count(*) FROM case_deadline_revisions),
        (SELECT count(*) FROM case_hearing_derived_deadline_origins),
        (SELECT count(*) FROM deadline_source_events WHERE source_kind='hearing_result'),
        (SELECT count(*) FROM audit_events WHERE action IN
            ('hearing_result.recorded','deadline.registered','hearing_derived_deadline.registered'))", &[]).unwrap();
    (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4))
}

#[test]
fn atomic_creation_reopens_and_replays_exactly_one_pair_and_source_event() {
    let Some(mut db) = Fixture::new() else { return };
    let draft = support::draft(&mut db);
    let workflow = service(&db);
    let command = draft.command().clone();
    let review = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    let HearingDerivedDeadlineReview::Ready(ready) = review else {
        panic!("new operation replayed")
    };
    assert_eq!(ready.review_digest(), draft.review_digest());
    assert_eq!(counts(&mut db), (0, 0, 0, 0, 0));
    let result = workflow
        .submit("session", db.case, command.clone(), ready.review_digest())
        .unwrap();
    assert_eq!(counts(&mut db), (1, 1, 1, 1, 3));
    drop(workflow);
    let reopened = service(&db);
    let HearingDerivedDeadlineReview::Replay(replay) = reopened
        .prepare("session", db.case, command.clone())
        .unwrap()
    else {
        panic!("stored operation was not replayed")
    };
    assert_eq!(*replay, result);
    assert_eq!(
        reopened
            .submit("session", db.case, command, ready.review_digest())
            .unwrap(),
        result
    );
    assert_eq!(counts(&mut db), (1, 1, 1, 1, 3));
}

#[test]
fn atomic_origin_failure_rolls_back_result_deadline_event_and_every_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let draft = support::draft(&mut db);
    let workflow = service(&db);
    let before: i64 = db
        .admin
        .query_one("SELECT count(*) FROM audit_events", &[])
        .unwrap()
        .get(0);
    db.admin.batch_execute("CREATE FUNCTION reject_compound_origin() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected origin failure'; END $$;
        CREATE TRIGGER reject_compound_origin BEFORE INSERT ON case_hearing_derived_deadline_origins FOR EACH ROW EXECUTE FUNCTION reject_compound_origin()").unwrap();
    assert!(workflow
        .submit(
            "session",
            db.case,
            draft.command().clone(),
            draft.review_digest()
        )
        .is_err());
    assert_eq!(counts(&mut db), (0, 0, 0, 0, 0));
    let after: i64 = db
        .admin
        .query_one("SELECT count(*) FROM audit_events", &[])
        .unwrap()
        .get(0);
    assert_eq!(after, before);
    db.admin.batch_execute("DROP TRIGGER reject_compound_origin ON case_hearing_derived_deadline_origins; DROP FUNCTION reject_compound_origin()").unwrap();
    db.admin.batch_execute("CREATE FUNCTION reject_compound_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='hearing_derived_deadline.registered' THEN RAISE EXCEPTION 'injected compound audit failure'; END IF; RETURN NEW; END $$;
        CREATE TRIGGER reject_compound_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_compound_audit()").unwrap();
    assert!(workflow
        .submit(
            "session",
            db.case,
            draft.command().clone(),
            draft.review_digest()
        )
        .is_err());
    assert_eq!(counts(&mut db), (0, 0, 0, 0, 0));
    let after: i64 = db
        .admin
        .query_one("SELECT count(*) FROM audit_events", &[])
        .unwrap()
        .get(0);
    assert_eq!(after, before);
    db.admin.batch_execute("DROP TRIGGER reject_compound_audit ON audit_events; DROP FUNCTION reject_compound_audit()").unwrap();
    workflow
        .submit(
            "session",
            db.case,
            draft.command().clone(),
            draft.review_digest(),
        )
        .unwrap();
    assert_eq!(counts(&mut db), (1, 1, 1, 1, 3));
}

#[test]
fn ordinary_components_without_origin_are_never_adopted_as_compound_success() {
    let Some(mut db) = Fixture::new() else { return };
    let ordinary = support::record(&mut db);
    let workflow = service(&db);
    let evidence = ordinary.evidence();
    let before = counts(&mut db);
    assert!(workflow
        .prepare("session", db.case, evidence.command.clone())
        .is_err());
    assert!(workflow
        .submit(
            "session",
            db.case,
            evidence.command.clone(),
            evidence.review_digest
        )
        .is_err());
    assert_eq!(counts(&mut db), before);
    assert_eq!(before, (1, 1, 0, 1, 2));
}

#[test]
fn reused_operation_with_changed_instruction_is_rejected_without_new_writes() {
    let Some(mut db) = Fixture::new() else { return };
    let draft = support::draft(&mut db);
    let workflow = service(&db);
    workflow
        .submit(
            "session",
            db.case,
            draft.command().clone(),
            draft.review_digest(),
        )
        .unwrap();
    let mut different = draft.command().clone();
    different.result.result_id = application::hearing_results::HearingResultId::new();
    assert!(workflow
        .prepare("session", db.case, different.clone())
        .is_err());
    assert!(workflow
        .submit("session", db.case, different, draft.review_digest())
        .is_err());
    let mut changed_values = draft.command().clone();
    let application::hearing_results::HearingResultChange::Record { ref mut values, .. } =
        changed_values.result.change
    else {
        unreachable!()
    };
    *values = crate::hearing_result_database_support::values("Different declared session");
    assert!(workflow
        .submit("session", db.case, changed_values, draft.review_digest())
        .is_err());
    assert_eq!(counts(&mut db), (1, 1, 1, 1, 3));
}

#[test]
fn concurrent_identical_submissions_return_one_committed_origin() {
    let Some(mut db) = Fixture::new() else { return };
    let draft = support::draft(&mut db);
    let first = service(&db);
    let second = service(&db);
    let barrier = std::sync::Barrier::new(2);
    let records = std::thread::scope(|scope| {
        let run = |workflow: &HearingDerivedDeadlineService| {
            barrier.wait();
            workflow
                .submit(
                    "session",
                    db.case,
                    draft.command().clone(),
                    draft.review_digest(),
                )
                .unwrap()
        };
        let a = scope.spawn(move || run(&first));
        let b = scope.spawn(move || run(&second));
        (a.join().unwrap(), b.join().unwrap())
    });
    assert_eq!(records.0, records.1);
    assert_eq!(counts(&mut db), (1, 1, 1, 1, 3));
}

#[test]
fn compound_origin_does_not_prevent_an_independent_manual_deadline() {
    use application::deadlines::*;
    let Some(mut db) = Fixture::new() else { return };
    let draft = support::draft(&mut db);
    let workflow = service(&db);
    workflow
        .submit(
            "session",
            db.case,
            draft.command().clone(),
            draft.review_digest(),
        )
        .unwrap();
    let (mut command, policies) = draft.command().deadline.clone().into_parts();
    command.operation_id = DeadlineOperationId::new();
    command.deadline_id = DeadlineId::new();
    let command = DeadlineHumanCommand::new(command, policies).unwrap();
    let ordinary = DeadlineService::new(
        Arc::new(
            infrastructure::PostgresDeadlineStore::open(
                &db.runtime_url,
                Arc::new(RingSha256Hasher),
                Arc::new(FixedClock(db.at)),
            )
            .unwrap(),
        ),
        Arc::new(TestIdentity(draft.actor().clone())),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    );
    let prepared = ordinary
        .prepare("session", db.case, command.clone())
        .unwrap();
    ordinary
        .submit("session", db.case, command, prepared.submission_digest)
        .unwrap();
    assert_eq!(counts(&mut db), (1, 2, 1, 1, 4));
}

#[test]
fn revoked_database_authority_rejects_fresh_capture_and_historical_replay() {
    let Some(mut db) = Fixture::new() else { return };
    db.user("owner", false);
    let draft = support::draft(&mut db);
    let workflow = service(&db);
    db.admin
        .execute(
            "UPDATE users SET role='paralegal',revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        workflow.submit(
            "session",
            db.case,
            draft.command().clone(),
            draft.review_digest()
        ),
        Err(application::ApplicationError::PermissionDenied)
    ));
    assert_eq!(counts(&mut db), (0, 0, 0, 0, 0));
    db.admin
        .execute(
            "UPDATE users SET role='owner',revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    workflow
        .submit(
            "session",
            db.case,
            draft.command().clone(),
            draft.review_digest(),
        )
        .unwrap();
    db.admin
        .execute(
            "UPDATE users SET role='paralegal',revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        workflow.prepare("session", db.case, draft.command().clone()),
        Err(application::ApplicationError::PermissionDenied)
    ));
    assert_eq!(counts(&mut db), (1, 1, 1, 1, 3));
}

#[test]
fn replay_keeps_the_original_profile_after_its_current_head_is_retired() {
    use crate::deadline_profile_database_support as profiles;
    use application::deadline_profiles::DeadlineProfileCollection;
    use domain::identity::Role;
    let Some(mut db) = Fixture::new() else { return };
    let draft = support::draft(&mut db);
    let workflow = service(&db);
    let original = workflow
        .submit(
            "session",
            db.case,
            draft.command().clone(),
            draft.review_digest(),
        )
        .unwrap();
    let retired = profiles::persist(
        &profiles::service(&db, db.owner, Role::Owner),
        DeadlineProfileCollection::ForCase(db.case),
        profiles::retire(&original.evidence().material.profile),
    );
    assert_eq!(retired.revision.get(), 2);
    drop(workflow);
    let reopened = service(&db);
    let HearingDerivedDeadlineReview::Replay(replay) = reopened
        .prepare("session", db.case, draft.command().clone())
        .unwrap()
    else {
        panic!("historical replay tried to prepare against the retired head")
    };
    assert_eq!(*replay, original);
    assert_eq!(
        reopened
            .submit(
                "session",
                db.case,
                draft.command().clone(),
                draft.review_digest()
            )
            .unwrap(),
        original
    );
    assert_eq!(counts(&mut db), (1, 1, 1, 1, 3));
}
