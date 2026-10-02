use super::password_reset_backend_support::*;
use super::password_reset_restore_support::*;
use domain::audit::AuditLog;
use infrastructure::PostgresAuditLog;
use postgres::{Client, NoTls};
use std::time::{Duration, Instant};

#[test]
fn the_table_owner_can_restore_in_a_database_owner_owned_namespace() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, target, 1);
    db.control
        .batch_execute(&format!(
            "ALTER SCHEMA {} OWNER TO pg_database_owner",
            db.schema,
        ))
        .unwrap();
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    let operation = request.operation_id;
    let result = invalidate_restored_password_resets(&db.admin_url, request).unwrap();
    assert!(result.applied);
    assert_eq!(result.invalidated, 1);
    assert_restore_receipt(&mut db, operation, &result);
}

#[test]
fn an_applied_operation_cannot_be_reconciled_against_a_different_predecessor() {
    let mut db = fixture();
    PostgresAuditLog::open(&db.runtime_url)
        .unwrap()
        .append("fixture", "restore.predecessor", "fixture", db.at)
        .unwrap();
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    assert!(request.expected_head.is_some());
    invalidate_restored_password_resets(&db.admin_url, repeat_request(&request)).unwrap();
    let confirmed = snapshot(&mut db);
    for head in [
        None,
        Some(PasswordResetRestoreHead {
            sequence: 0,
            chain: digest(250),
        }),
    ] {
        let mut changed = repeat_request(&request);
        changed.expected_head = head;
        assert!(invalidate_restored_password_resets(&db.admin_url, changed).is_err());
        assert_private_unchanged(&mut db, &confirmed);
    }
    let result = invalidate_restored_password_resets(&db.admin_url, request).unwrap();
    assert!(!result.applied);
    assert_private_unchanged(&mut db, &confirmed);
}

#[test]
fn altered_authority_or_audit_is_rejected_without_schema_repair_or_cancellation() {
    for condition in ["consume", "counter_guard", "acl", "chain"] {
        let mut db = fixture();
        let target = account(&mut db, "litigator", true);
        let repository = store(&db);
        issue(&repository, target, 1);
        PostgresAuditLog::open(&db.runtime_url)
            .unwrap()
            .append("fixture", "restore.catalog", "fixture", db.at)
            .unwrap();
        let request = restore_request(&mut db, uuid::Uuid::new_v4());
        let signature = match condition {
            "consume" => Some("password_reset_consume(uuid,bytea,uuid,bigint,text)"),
            "counter_guard" => Some("guard_member_access()"),
            _ => None,
        };
        if let Some(signature) = signature {
            db.admin
                .batch_execute(&format!(
                    "ALTER FUNCTION {signature} SET search_path=public"
                ))
                .unwrap();
        } else if condition == "acl" {
            db.admin
                .batch_execute(&format!(
                    "GRANT SELECT ON password_reset_capabilities TO {}",
                    db.role
                ))
                .unwrap();
        } else {
            db.admin
                .execute(
                    "UPDATE audit_events SET resource='altered' WHERE action='restore.catalog'",
                    &[],
                )
                .unwrap();
        }
        let before = snapshot(&mut db);
        assert!(
            invalidate_restored_password_resets(&db.admin_url, request).is_err(),
            "{condition}"
        );
        assert_private_unchanged(&mut db, &before);
        if let Some(signature) = signature {
            let config: Option<Vec<String>> = db
                .admin
                .query_one(
                    "SELECT proconfig FROM pg_proc WHERE oid=$1::text::regprocedure",
                    &[&signature],
                )
                .unwrap()
                .get(0);
            assert_eq!(config, Some(vec!["search_path=public".into()]));
        } else if condition == "acl" {
            let granted: bool = db
                .admin
                .query_one(
                    "SELECT has_table_privilege($1,'password_reset_capabilities','SELECT')",
                    &[&db.role],
                )
                .unwrap()
                .get(0);
            assert!(
                granted,
                "administrative operation unexpectedly repaired runtime ACL"
            );
        }
    }
}

#[test]
fn cancellation_clock_is_read_after_waiting_for_the_pending_row() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    let issued = issue(&repository, target, 1);
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    let operation = request.operation_id;
    let application = format!("reset_restore_row_{}", uuid::Uuid::new_v4().simple());
    let url = operation_url(&db, &application, None);
    let mut blocker = Client::connect(&db.admin_url, NoTls).unwrap();
    let pid: i32 = blocker
        .query_one("SELECT pg_backend_pid()", &[])
        .unwrap()
        .get(0);
    let mut held = blocker.transaction().unwrap();
    held.query_one(
        "SELECT id FROM password_reset_capabilities WHERE id=$1 FOR UPDATE",
        &[&issued.id.as_uuid()],
    )
    .unwrap();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = sender.send(invalidate_restored_password_resets(&url, request));
    });
    let deadline = Instant::now() + Duration::from_secs(8);
    let reached_row = loop {
        let waiting: bool = db
            .admin
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1
             AND wait_event_type='Lock' AND wait_event='transactionid'
             AND $2=ANY(pg_blocking_pids(pid)))",
                &[&application, &pid],
            )
            .unwrap()
            .get(0);
        if waiting || Instant::now() >= deadline {
            break waiting;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    held.execute(
        "UPDATE password_reset_capabilities SET issued_at=clock_timestamp(),
        expires_at=clock_timestamp()+interval '1 minute' WHERE id=$1",
        &[&issued.id.as_uuid()],
    )
    .unwrap();
    held.commit().unwrap();
    let result = receiver.recv_timeout(Duration::from_secs(8));
    worker.join().unwrap();
    assert!(
        reached_row,
        "invalidation did not wait for the pending row lock"
    );
    let result = result.unwrap().unwrap();
    assert!(result.applied);
    assert_eq!(result.invalidated, 1);
    let time_valid: bool = db
        .admin
        .query_one(
            "SELECT cancelled_at>=issued_at
        FROM password_reset_capabilities WHERE id=$1",
            &[&issued.id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert!(time_valid);
    assert_restore_receipt(&mut db, operation, &result);
}
