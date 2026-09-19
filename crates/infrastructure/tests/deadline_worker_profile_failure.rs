mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
#[allow(dead_code)]
mod deadline_worker_backend_support;
#[allow(dead_code)]
mod deadline_worker_guard_support;
mod procedural_fact_backend_support;

use application::{
    deadline_profiles::*,
    deadline_worker::{
        DeadlineWorkerErrorCode, DeadlineWorkerFailureKind, DeadlineWorkerRun, DeadlineWorkerStore,
    },
    ApplicationError,
};
use deadline_backend_support as dl;
use deadline_profile_database_support as profiles;
use deadline_worker_backend_support as worker;
use deadline_worker_guard_support as guard;
use domain::{crypto::DocumentHasher, identity::Role};
use infrastructure::RingSha256Hasher;
use time::Duration;

#[test]
fn corrupt_newer_profile_head_records_integrity_failure_with_one_hour_retry() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let setup = guard::setup(&mut db);
    let selected = &setup.base.calculation.profile;
    let mut values = profiles::input(Some(db.case));
    values.title = dl::label("Revised synthetic profile");
    let definition = DeadlineProfileDefinition::new(values).unwrap();
    assert_ne!(definition, selected.definition);
    let collection = DeadlineProfileCollection::ForCase(db.case);
    let head = profiles::persist(
        &profiles::service(&db, db.owner, Role::Owner),
        collection,
        DeadlineProfileCommand {
            operation_id: DeadlineProfileOperationId::new(),
            profile_id: selected.id,
            change: DeadlineProfileChange::Replace {
                expected_revision: selected.revision,
                definition,
                reason: profiles::text("Replace the mathematical fixture"),
            },
        },
    );
    assert_eq!(head.revision.get(), selected.revision.get() + 1);
    // Both connections open against valid history. The job authenticates a
    // resolution event, while its deadline still captures profile revision one.
    let profile_reader = profiles::store(&db);
    let runner = worker::open(&db);
    let original: Vec<u8> = db.admin.query_one(
        "SELECT submission_canonical FROM deadline_profile_revisions WHERE profile_id=$1 AND revision=$2",
        &[&head.id.as_uuid(), &i64::from(head.revision.get())],
    ).unwrap().get(0);
    let forged = DeadlineProfileCommand {
        operation_id: head.receipt.operation_id,
        profile_id: head.id,
        change: DeadlineProfileChange::Retire {
            expected_revision: selected.revision,
            reason: head.reason.clone().unwrap(),
        },
    };
    let bytes = deadline_profile_submission_bytes(
        head.recorded_by.id,
        &forged,
        head.algorithm,
        head.definition_digest,
    );
    // A retirement cannot replace the prior definition. Canonical hashes and
    // SQL projections remain internally consistent, so the historical reader
    // must detect the invalid successor after all SQL guards are restored.
    replace_submission(&mut db, &head, "retire", &bytes);
    assert!(matches!(
        profile_reader.get(db.owner, collection, head.id, None, db.at),
        Err(ApplicationError::DeadlineProfile(
            DeadlineProfileError::StoredInconsistent(_)
        ))
    ));
    assert_eq!(
        profile_reader
            .get(
                db.owner,
                collection,
                selected.id,
                Some(selected.revision),
                db.at
            )
            .unwrap(),
        *selected
    );
    let before = guard::data_snapshot(&mut db);
    let audits = guard::audit_count(&mut db);
    let outcome = runner.run_next();
    // Restore only the injected bytes before asserting the failure category.
    replace_submission(&mut db, &head, "replace", &original);
    let DeadlineWorkerRun::Deferred(attempt) = outcome.unwrap() else {
        panic!("a verified base and corrupt newer head must produce a durable failure")
    };
    assert_eq!(guard::data_snapshot(&mut db), before);
    guard::assert_attempt(&mut db, &setup, &attempt, audits);
    assert_eq!(
        attempt.failure_kind,
        DeadlineWorkerFailureKind::Inconsistent
    );
    assert_eq!(
        attempt.error_code,
        DeadlineWorkerErrorCode::InvalidStoredEvidence
    );
    assert_eq!(attempt.failed_at, db.at);
    assert_eq!(
        attempt.retry_at,
        attempt.failed_at + Duration::seconds(3600)
    );
    let reopened = worker::open(&db);
    assert_eq!(
        reopened.latest_attempt(setup.job.id).unwrap(),
        Some(attempt.clone())
    );
    assert!(reopened.result(setup.job.id).unwrap().is_none());
    worker::assert_idle(&mut db, &reopened);
    assert_eq!(worker::current(&db, setup.base.id), setup.base);
    let due = guard::open_at(&db, attempt.retry_at);
    assert!(matches!(
        due.run_next().unwrap(),
        DeadlineWorkerRun::Completed(_)
    ));
    assert_eq!(due.latest_attempt(setup.job.id).unwrap(), Some(attempt));
}

fn replace_submission(
    db: &mut dl::Fixture,
    head: &DeadlineProfileDetail,
    action: &str,
    bytes: &[u8],
) {
    let digest = RingSha256Hasher.hash_bytes(bytes);
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute(
        "ALTER TABLE deadline_profile_revisions DISABLE TRIGGER deadline_profile_immutable",
    )
    .unwrap();
    assert_eq!(tx.execute(
        "UPDATE deadline_profile_revisions SET action=$1,submission_canonical=$2,submission_digest=$3
         WHERE profile_id=$4 AND revision=$5",
        &[&action, &bytes, &digest.as_bytes().as_slice(), &head.id.as_uuid(), &i64::from(head.revision.get())],
    ).unwrap(), 1);
    tx.batch_execute(
        "ALTER TABLE deadline_profile_revisions ENABLE TRIGGER deadline_profile_immutable",
    )
    .unwrap();
    tx.commit().unwrap();
}
