use super::password_reset_backend_support::*;
use super::password_reset_restore_support::*;

#[test]
fn restored_pending_links_are_cancelled_without_changing_identity_or_consumed_receipts() {
    let mut db = fixture();
    let repository = store(&db);
    let inventory = populate(&mut db, &repository);
    let original = snapshot(&mut db);
    let business = db.snapshot();
    drop(repository);
    dump_restore(&mut db);
    assert_private_unchanged(&mut db, &original);
    assert!(
        db.snapshot() == business,
        "restoration changed business rows or audit"
    );

    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    let operation = request.operation_id;
    let result = invalidate_restored_password_resets(&db.admin_url, request).unwrap();
    assert!(result.applied);
    assert_eq!(result.invalidated, 4);
    let after = snapshot(&mut db);
    for table in ["users", "members"] {
        assert!(
            after[table] == original[table],
            "restoration invalidation changed identity"
        );
    }
    let old_rows = original["capabilities"].as_array().unwrap();
    let new_rows = after["capabilities"].as_array().unwrap();
    assert_eq!(old_rows.len(), new_rows.len());
    for (old, new) in old_rows.iter().zip(new_rows) {
        if old["consumed_at"].is_null() && old["cancelled_at"].is_null() {
            assert!(
                !new["cancelled_at"].is_null(),
                "pending capability survived invalidation"
            );
            let mut unchanged = new.clone();
            unchanged["cancelled_at"] = serde_json::Value::Null;
            assert!(
                unchanged == *old,
                "invalidation changed more than cancelled_at"
            );
        } else {
            assert!(
                new == old,
                "terminal capability or consumed receipt changed"
            );
        }
    }
    let pending: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM password_reset_capabilities
        WHERE consumed_at IS NULL AND cancelled_at IS NULL",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(pending, 0);
    let same_time: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM password_reset_capabilities r
        JOIN audit_events a ON r.cancelled_at=a.timestamp::timestamptz WHERE a.sequence=$1",
            &[&i64::try_from(result.audit_sequence).unwrap()],
        )
        .unwrap()
        .get(0);
    assert_eq!(same_time, 4);
    let unchanged_business = db.snapshot();
    for (table, rows) in business.as_object().unwrap() {
        if table != "audit" {
            assert!(
                unchanged_business[table] == *rows,
                "business rows changed during invalidation"
            );
        }
    }
    assert_restore_receipt(&mut db, operation, &result);

    let repository = store(&db);
    let cancelled = snapshot(&mut db);
    for (value, candidate) in inventory.candidates {
        assert!(repository.inspect(digest(value)).unwrap().is_none());
        assert_eq!(
            repository
                .complete(complete(candidate, value, "must-not-change"))
                .unwrap(),
            ResetCompletion::Rejected
        );
    }
    assert_private_unchanged(&mut db, &cancelled);
    issue(&repository, inventory.live, 7);
    assert!(repository.inspect(digest(7)).unwrap().is_some());
    assert_chain(&db);
}

#[test]
fn restoration_retry_returns_the_original_receipt_but_cannot_cancel_later_issuance() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, target, 1);
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    let first =
        invalidate_restored_password_resets(&db.admin_url, repeat_request(&request)).unwrap();
    let confirmed = snapshot(&mut db);
    let retry =
        invalidate_restored_password_resets(&db.admin_url, repeat_request(&request)).unwrap();
    assert!(first.applied);
    assert!(!retry.applied);
    assert_eq!(retry.invalidated, 1);
    assert_eq!(retry.audit_sequence, first.audit_sequence);
    assert_eq!(retry.audit_head, first.audit_head);
    assert_private_unchanged(&mut db, &confirmed);
    assert_restore_receipt(&mut db, request.operation_id, &retry);

    issue(&repository, target, 2);
    let with_new_issue = snapshot(&mut db);
    assert!(invalidate_restored_password_resets(&db.admin_url, repeat_request(&request)).is_err());
    assert_private_unchanged(&mut db, &with_new_issue);
    assert!(repository.inspect(digest(2)).unwrap().is_some());
}

#[test]
fn an_empty_restoration_still_gets_one_durable_idempotent_receipt() {
    let mut db = fixture();
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    let first =
        invalidate_restored_password_resets(&db.admin_url, repeat_request(&request)).unwrap();
    assert!(first.applied);
    assert_eq!(first.invalidated, 0);
    assert_restore_receipt(&mut db, request.operation_id, &first);
    let confirmed = snapshot(&mut db);
    let retry =
        invalidate_restored_password_resets(&db.admin_url, repeat_request(&request)).unwrap();
    assert!(!retry.applied);
    assert_eq!(retry.invalidated, 0);
    assert_eq!(retry.audit_sequence, first.audit_sequence);
    assert_eq!(retry.audit_head, first.audit_head);
    assert_private_unchanged(&mut db, &confirmed);
}

#[test]
fn runtime_wrong_target_stale_head_and_nil_identity_cannot_invalidate() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, target, 1);
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    let before = snapshot(&mut db);
    assert!(
        invalidate_restored_password_resets(&db.runtime_url, repeat_request(&request)).is_err()
    );
    assert_private_unchanged(&mut db, &before);
    for condition in ["database", "schema", "head", "nil"] {
        let mut changed = repeat_request(&request);
        match condition {
            "database" => changed.expected_database.push_str("_unrelated"),
            "schema" => changed.expected_schema.push_str("_unrelated"),
            "head" => {
                changed.expected_head = Some(PasswordResetRestoreHead {
                    sequence: u64::MAX,
                    chain: digest(255),
                })
            }
            "nil" => changed.operation_id = uuid::Uuid::nil(),
            _ => unreachable!(),
        }
        assert!(
            invalidate_restored_password_resets(&db.admin_url, changed).is_err(),
            "{condition}"
        );
        assert_private_unchanged(&mut db, &before);
    }
    let mut runtime = db.runtime();
    for statement in [
        "SELECT * FROM password_reset_capabilities",
        "UPDATE password_reset_capabilities SET cancelled_at=clock_timestamp()",
        "SELECT password_reset_audit_receipt()",
    ] {
        assert!(
            runtime.batch_execute(statement).is_err(),
            "runtime privilege increased"
        );
    }
    let execute: bool = db
        .admin
        .query_one(
            "SELECT has_function_privilege($1,'password_reset_audit_receipt()','EXECUTE')",
            &[&db.role],
        )
        .unwrap()
        .get(0);
    assert!(!execute);
    assert_private_unchanged(&mut db, &before);
    assert!(repository.inspect(digest(1)).unwrap().is_some());
}

#[test]
fn a_future_issued_capability_rejects_the_whole_invalidation_without_clock_forgery() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, target, 1);
    let future = issue(&repository, target, 2);
    db.admin
        .execute(
            "UPDATE password_reset_capabilities SET
        issued_at=clock_timestamp()+interval '1 hour',
        expires_at=clock_timestamp()+interval '2 hours' WHERE id=$1",
            &[&future.id.as_uuid()],
        )
        .unwrap();
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    let before = snapshot(&mut db);
    assert!(invalidate_restored_password_resets(&db.admin_url, repeat_request(&request)).is_err());
    assert_private_unchanged(&mut db, &before);
    db.admin
        .execute(
            "UPDATE password_reset_capabilities SET
        issued_at=clock_timestamp()-interval '1 second' WHERE id=$1",
            &[&future.id.as_uuid()],
        )
        .unwrap();
    let result = invalidate_restored_password_resets(&db.admin_url, request).unwrap();
    assert!(result.applied);
    assert_eq!(result.invalidated, 2);
    assert_chain(&db);
}
