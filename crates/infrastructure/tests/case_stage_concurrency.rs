mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;

use application::case_stages::*;
use application::ApplicationError;
use case_stage_database_support::*;
use domain::identity::Role;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

fn wait_for_lock(client: &mut impl postgres::GenericClient, role: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let waiting:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename=$1 AND wait_event='advisory')",&[&role]).unwrap().get(0);
        if waiting {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "stage operation did not wait for the audit lock"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn competing_adoptions_and_transitions_have_exactly_one_audited_successor() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let record = upload(&db, db.case, "support.pdf");
    for transition in [false, true] {
        let barrier = Arc::new(Barrier::new(2));
        let workers: Vec<_> = (0..2)
            .map(|_| {
                let barrier = barrier.clone();
                let workflow = service(
                    &db,
                    db.owner,
                    Role::Owner,
                    FormatCheck(Some(Box::new(move || {
                        barrier.wait();
                    }))),
                );
                let adoption = adoption(&db, &record, CaseStage::Investigation);
                let value = StageTransition::to_intermediate(
                    DeclaredStageTime::instant(db.at).unwrap(),
                    reference(&record),
                    None,
                );
                let case = db.case;
                std::thread::spawn(move || {
                    if transition {
                        workflow.transition("session", case, CaseStageRevision::FIRST, value)
                    } else {
                        workflow.adopt(
                            "session",
                            case,
                            CaseStageExpectation::Unregistered,
                            adoption,
                        )
                    }
                })
            })
            .collect();
        let results: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Err(ApplicationError::CaseStageConflict)))
                .count(),
            1
        );
    }
    let row=db.admin.query_one("SELECT (SELECT count(*) FROM case_stage_revisions),(SELECT count(*) FROM audit_events WHERE action IN ('case.stage_adopted','case.stage_transitioned'))",&[]).unwrap();
    assert_eq!(row.get::<_, i64>(0), 2);
    assert_eq!(row.get::<_, i64>(1), 2);
}

#[test]
fn waiting_read_uses_read_committed_even_when_connection_default_is_repeatable_read() {
    let Some(mut db) = Fixture::new() else { return };
    let actor = db.user("litigator", true);
    db.admin
        .batch_execute(&format!(
            "ALTER ROLE {} SET default_transaction_isolation='repeatable read'",
            db.role
        ))
        .unwrap();
    let stages = store(&db);
    let before = snapshot(&mut db);
    db.admin
        .batch_execute("SELECT pg_advisory_lock(280603412820)")
        .unwrap();
    let (case, at) = (db.case, db.at);
    let worker = std::thread::spawn(move || stages.get(actor, case, at));
    wait_for_lock(&mut db.admin, &db.role);
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&case.as_uuid(), &actor.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("SELECT pg_advisory_unlock(280603412820)")
        .unwrap();
    assert!(matches!(
        worker.join().unwrap(),
        Err(ApplicationError::CaseNotFound)
    ));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn direct_sql_successor_observes_prior_commit_after_lock_and_refuses_prior_rollback() {
    for commit in [true, false] {
        let Some(mut db) = Fixture::new() else { return };
        complete(&db);
        let record = upload(&db, db.case, "support.pdf");
        let mut value = row_value(&db, &record);
        let first = signed_value(&mut db.admin, &value);
        value["revision"] = 2.into();
        value["change_kind"] = "to_intermediate".into();
        value["from_stage"] = "investigation".into();
        value["stage"] = "intermediate".into();
        value["reason"] = serde_json::Value::Null;
        let second = signed_value(&mut db.admin, &value);
        let mut runtime = db.runtime();
        let mut tx = db.admin.transaction().unwrap();
        tx.execute(INSERT, &[&first]).unwrap();
        let worker = std::thread::spawn(move || runtime.execute(INSERT, &[&second]));
        wait_for_lock(&mut tx, &db.role);
        if commit {
            tx.commit().unwrap();
        } else {
            tx.rollback().unwrap();
        }
        let result = worker.join().unwrap();
        if commit {
            assert_eq!(result.unwrap(), 1);
        } else {
            assert_eq!(result.unwrap_err().code().unwrap().code(), "23514");
        }
        let count: i64 = db
            .admin
            .query_one("SELECT count(*) FROM case_stage_revisions", &[])
            .unwrap()
            .get(0);
        assert_eq!(count, if commit { 2 } else { 0 });
    }
}
