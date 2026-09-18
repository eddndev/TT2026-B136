mod case_administration_support;
mod deadline_responsibles_support;
use application::{deadlines::*, ApplicationError};
use deadline_responsibles_support::*;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

#[test]
fn waiting_responsible_reads_see_actor_and_candidate_revocations_after_audit_lock() {
    let Some(mut db) = Fixture::new() else { return };
    db.admin.batch_execute(&format!(
        "ALTER ROLE {} SET default_transaction_isolation='repeatable read'; ALTER ROLE {} SET statement_timeout='8s'",
        db.role, db.role
    )).unwrap();
    for change in 0..4 {
        let actor = db.user("paralegal", true);
        let candidate = db.user("litigator", true);
        let adapter = store(&db);
        let (case, at) = (db.case, db.at);
        let before: i64 = db
            .admin
            .query_one("SELECT count(*) FROM audit_events", &[])
            .unwrap()
            .get(0);
        let mut gate = db.control.transaction().unwrap();
        gate.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])
            .unwrap();
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            sender
                .send(adapter.responsibles(
                    actor,
                    case,
                    DeadlineResponsibleQuery::new(100, None).unwrap(),
                    at,
                ))
                .unwrap();
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        let observed = loop {
            let waiting: bool = db.admin.query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename=$1 AND wait_event='advisory')",
                &[&db.role]
            ).unwrap().get(0);
            if waiting {
                break true;
            }
            if Instant::now() >= deadline {
                break false;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        match change {
            0 => {
                db.admin
                    .execute(
                        "UPDATE users SET active=false WHERE id=$1",
                        &[&actor.as_uuid()],
                    )
                    .unwrap();
            }
            1 => {
                db.admin
                    .execute(
                        "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                        &[&case.as_uuid(), &actor.as_uuid()],
                    )
                    .unwrap();
            }
            2 => {
                db.admin
                    .execute(
                        "UPDATE users SET active=false WHERE id=$1",
                        &[&candidate.as_uuid()],
                    )
                    .unwrap();
            }
            _ => {
                db.admin
                    .execute(
                        "UPDATE users SET role='client' WHERE id=$1",
                        &[&candidate.as_uuid()],
                    )
                    .unwrap();
            }
        }
        gate.commit().unwrap();
        let result = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("responsible read did not finish after releasing the audit lock");
        worker.join().unwrap();
        assert!(observed, "responsible read did not wait for audit lock");
        match change {
            0 => assert!(matches!(result, Err(ApplicationError::InvalidSession))),
            1 => assert!(matches!(result, Err(ApplicationError::CaseNotFound))),
            _ => assert!(!result
                .unwrap()
                .responsibles
                .iter()
                .any(|row| row.id == candidate)),
        }
        let after: i64 = db
            .admin
            .query_one("SELECT count(*) FROM audit_events", &[])
            .unwrap()
            .get(0);
        assert_eq!(after, before + i64::from(change >= 2));
    }
}
