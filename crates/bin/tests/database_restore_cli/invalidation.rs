use crate::password_reset_backend_support::*;
use crate::password_reset_restore_support::*;
use crate::support::*;

#[test]
fn explicit_predecessor_cancels_pending_and_matches_the_existing_primitive_receipt() {
    let mut db = fixture();
    let repository = store(&db);
    populate(&mut db, &repository);
    let before = snapshot(&mut db);
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    assert!(
        request.expected_head.is_some(),
        "fixture needs a nonempty predecessor"
    );
    let first = success(invoke(Some(&db.admin_url), &restore_args(&request)));
    let reconciled = invalidate_restored_password_resets(&db.admin_url, request.clone()).unwrap();
    assert!(!reconciled.applied);
    assert_eq!(reconciled.invalidated, 4);
    let mut expected = expected_receipt(request.operation_id, reconciled);
    expected["applied"] = serde_json::Value::Bool(true);
    assert_eq!(first, expected);
    assert_restore_receipt(&mut db, request.operation_id, &reconciled);
    let after = snapshot(&mut db);
    for key in ["users", "members"] {
        assert!(
            after[key] == before[key],
            "invalidation changed identity state"
        );
    }
    for (old, new) in before["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .zip(after["capabilities"].as_array().unwrap())
    {
        if old["consumed_at"].is_null() && old["cancelled_at"].is_null() {
            assert!(!new["cancelled_at"].is_null());
            let mut unchanged = new.clone();
            unchanged["cancelled_at"] = serde_json::Value::Null;
            assert!(
                unchanged == *old,
                "invalidation changed a pending capability payload"
            );
        } else {
            assert!(new == old, "invalidation changed a terminal capability");
        }
    }
    let confirmed = state(&mut db);
    let retried = success(invoke(Some(&db.admin_url), &restore_args(&request)));
    assert_eq!(retried, expected_receipt(request.operation_id, reconciled));
    unchanged(&mut db, &confirmed);
}

#[test]
fn runtime_wrong_target_and_wrong_predecessor_never_emit_receipts_or_modify_state() {
    let mut db = fixture();
    let user = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, user, 1);
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    assert!(request.expected_head.is_none());
    let before = state(&mut db);
    rejected(invoke(Some(&db.runtime_url), &restore_args(&request)));
    unchanged(&mut db, &before);
    for condition in ["database", "schema", "head"] {
        let mut wrong = request.clone();
        match condition {
            "database" => wrong.expected_database.push_str("_different"),
            "schema" => wrong.expected_schema.push_str("_different"),
            "head" => {
                wrong.expected_head = Some(PasswordResetRestoreHead {
                    sequence: 0,
                    chain: digest(5),
                })
            }
            _ => unreachable!(),
        }
        rejected(invoke(Some(&db.admin_url), &restore_args(&wrong)));
        unchanged(&mut db, &before);
    }
}

#[test]
fn lost_acknowledgement_reuses_the_saved_empty_predecessor_and_cannot_cancel_new_issuance() {
    let mut db = fixture();
    let user = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, user, 1);
    let saved = restore_request(&mut db, uuid::Uuid::new_v4());
    assert!(saved.expected_head.is_none());
    // A committed primitive call represents a lost controller acknowledgement.
    let mut committed = invalidate_restored_password_resets(&db.admin_url, saved.clone()).unwrap();
    committed.applied = false;
    let before = state(&mut db);
    let retry = success(invoke(Some(&db.admin_url), &restore_args(&saved)));
    assert_eq!(retry, expected_receipt(saved.operation_id, committed));
    unchanged(&mut db, &before);
    let mut changed = saved.clone();
    changed.expected_head = Some(PasswordResetRestoreHead {
        sequence: committed.audit_sequence,
        chain: committed.audit_head,
    });
    rejected(invoke(Some(&db.admin_url), &restore_args(&changed)));
    unchanged(&mut db, &before);
    issue(&repository, user, 2);
    let newly_issued = state(&mut db);
    rejected(invoke(Some(&db.admin_url), &restore_args(&saved)));
    unchanged(&mut db, &newly_issued);
    assert!(repository.inspect(digest(2)).unwrap().is_some());
    assert_restore_receipt(&mut db, saved.operation_id, &committed);
}
