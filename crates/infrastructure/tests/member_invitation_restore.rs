#[path = "member_invitation_restore/backup.rs"]
mod backup;
#[path = "case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "member_invitation_restore/support.rs"]
mod support;

use application::identity::invitations::*;
use domain::audit::{verify_chain, AuditLog, ChainVerification};
use domain::clock::OffsetDateTime;
use infrastructure::{
    identity::{invalidate_restored_invitations, PasswordResetRestoreHead},
    PostgresAuditLog, PostgresInvitationStore, RingSha256Hasher,
};
use postgres::{Client, NoTls};
use std::time::{Duration, Instant};
use support::*;

#[test]
fn restored_pending_links_mfa_and_mail_are_invalidated_without_changing_accepted_accounts() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let plain = issue(&db, &store, 1);
    let mfa = issue(&db, &store, 2);
    let pending = store.save_pending(preparation(&mfa, 2)).unwrap();
    let accepted = issue(&db, &store, 3);
    let accepted_pending = store.save_pending(preparation(&accepted, 3)).unwrap();
    let receipt = store
        .finish(finish(&accepted, &accepted_pending, 3))
        .unwrap();
    let accepted_before = store.get(actor(&db), accepted.id).unwrap();
    let lease = store.claim_delivery(claim()).unwrap().unwrap();
    let restore = request(&mut db);
    let original = snapshot(&mut db);

    let result = invalidate_restored_invitations(&db.admin_url, restore).unwrap();
    assert!(result.applied);
    assert_eq!(result.invalidated, 2);
    assert_receipt(&mut db, result);
    let after = snapshot(&mut db);
    for table in ["users", "operations", "completions"] {
        assert!(
            after[table] == original[table],
            "restoration changed {table}"
        );
    }
    for invitation in [&plain, &mfa] {
        let cleared: bool = db
            .admin
            .query_one(
                "SELECT cancelled_at IS NOT NULL AND protected_token IS NULL
                AND pending_operation IS NULL AND pending_digest IS NULL
                AND pending_material IS NULL AND lease_id IS NULL AND lease_until IS NULL
             FROM member_invitations WHERE id=$1",
                &[&invitation.id.as_uuid()],
            )
            .unwrap()
            .get(0);
        assert!(
            cleared,
            "restored invitation retained pending secrets or delivery lease"
        );
        assert_eq!(
            store.get(actor(&db), invitation.id).unwrap().status,
            InvitationStatus::Expired
        );
    }
    assert!(store.inspect(digest(1)).unwrap().is_none());
    assert!(store.inspect(digest(2)).unwrap().is_none());
    assert!(store.save_pending(preparation(&mfa, 2)).is_err());
    assert!(store.finish(finish(&mfa, &pending, 2)).is_err());
    assert!(store.claim_delivery(claim()).unwrap().is_none());
    assert!(!store
        .finish_delivery(InvitationDeliveryFinish {
            id: lease.id,
            revision: lease.revision,
            generation: lease.generation,
            lease_id: lease.lease_id,
            state: InvitationDeliveryState::Sent,
            retry_at: None,
            at: OffsetDateTime::now_utc(),
        })
        .unwrap());
    assert_eq!(store.get(actor(&db), accepted.id).unwrap(), accepted_before);
    assert_eq!(
        store
            .completed(digest(3), receipt.operation_id)
            .unwrap()
            .unwrap()
            .receipt,
        receipt,
    );
    PostgresInvitationStore::open(&db.runtime_url).unwrap();
}

#[test]
fn restoration_retry_returns_its_receipt_and_cannot_cancel_later_resend_or_issuance() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let first = issue(&db, &store, 1);
    store.save_pending(preparation(&first, 1)).unwrap();
    let restore = request(&mut db);
    let applied = invalidate_restored_invitations(&db.admin_url, restore.clone()).unwrap();
    let original = snapshot(&mut db);
    let retry = invalidate_restored_invitations(&db.admin_url, restore.clone()).unwrap();
    assert!(applied.applied);
    assert!(!retry.applied);
    assert_eq!(retry.invalidated, applied.invalidated);
    assert_eq!(retry.audit_sequence, applied.audit_sequence);
    assert_eq!(retry.audit_head, applied.audit_head);
    assert!(
        snapshot(&mut db) == original,
        "retry changed committed restoration state"
    );
    assert_receipt(&mut db, retry);

    let resent = store
        .resend(
            actor(&db),
            first.id,
            ResendInvitation {
                request: InvitationResendRequest {
                    operation_id: Uuid::new_v4(),
                    expected_revision: first.revision,
                },
                token_digest: digest(4),
                protected_token: vec![4; 80],
                at: OffsetDateTime::now_utc(),
            },
        )
        .unwrap();
    let later = issue(&db, &store, 2);
    assert_eq!(resent.revision, first.revision + 1);
    assert_eq!(resent.status, InvitationStatus::Pending);
    let current = store.inspect(digest(4)).unwrap().unwrap();
    assert!(current.pending.is_none());
    assert_eq!(current.id, first.id);
    assert_eq!(store.inspect(digest(2)).unwrap().unwrap().id, later.id);
    let before_retry = snapshot(&mut db);
    assert!(invalidate_restored_invitations(&db.admin_url, restore).is_err());
    assert!(
        snapshot(&mut db) == before_retry,
        "old restoration changed later invitations"
    );
    assert!(store.inspect(digest(4)).unwrap().is_some());
    assert!(store.inspect(digest(2)).unwrap().is_some());
    assert!(store.claim_delivery(claim()).unwrap().is_some());
    assert_receipt(&mut db, retry);
}

#[test]
fn runtime_wrong_target_stale_head_and_nil_operation_cannot_invalidate_invitations() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    issue(&db, &store, 1);
    let restore = request(&mut db);
    let before = snapshot(&mut db);
    assert!(invalidate_restored_invitations(&db.runtime_url, restore.clone()).is_err());
    assert!(
        snapshot(&mut db) == before,
        "runtime invalidated invitation state"
    );
    for condition in ["database", "schema", "head", "nil"] {
        let mut changed = restore.clone();
        match condition {
            "database" => changed.expected_database.push_str("_unrelated"),
            "schema" => changed.expected_schema.push_str("_unrelated"),
            "head" => {
                changed.expected_head = Some(PasswordResetRestoreHead {
                    sequence: 0,
                    chain: digest(255),
                })
            }
            "nil" => changed.operation_id = Uuid::nil(),
            _ => unreachable!(),
        }
        assert!(
            invalidate_restored_invitations(&db.admin_url, changed).is_err(),
            "{condition}"
        );
        assert!(
            snapshot(&mut db) == before,
            "rejected target changed invitation state"
        );
    }
    assert!(store.inspect(digest(1)).unwrap().is_some());
}

#[test]
fn failed_restoration_audit_rolls_back_invitation_and_pending_mfa_invalidation() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let first = issue(&db, &store, 1);
    store.save_pending(preparation(&first, 1)).unwrap();
    let restore = request(&mut db);
    let before = snapshot(&mut db);
    let application = format!("invitation_restore_{}", Uuid::new_v4().simple());
    let url = operation_url(&db, &application);
    let attempted = restore.clone();
    let mut blocker = Client::connect(&db.admin_url, NoTls).unwrap();
    let mut held = blocker.transaction().unwrap();
    // SHARE admits validation reads but blocks the later audit INSERT.
    held.batch_execute("LOCK TABLE audit_events IN SHARE MODE")
        .unwrap();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = sender.send(invalidate_restored_invitations(&url, attempted));
    });
    let deadline = Instant::now() + Duration::from_secs(8);
    let reached_audit = loop {
        let waiting: bool = db
            .admin
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1
                 AND wait_event_type='Lock' AND wait_event='relation'
                 AND query ILIKE '%INSERT%audit_events%')",
                &[&application],
            )
            .unwrap()
            .get(0);
        if waiting || Instant::now() >= deadline {
            break waiting;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let failed = receiver.recv_timeout(Duration::from_secs(8));
    held.rollback().unwrap();
    worker.join().unwrap();
    assert!(
        reached_audit,
        "operation did not reach the blocked audit INSERT"
    );
    assert!(
        failed.unwrap().is_err(),
        "audit timeout unexpectedly committed"
    );
    assert!(
        snapshot(&mut db) == before,
        "failed audit left partial invitation invalidation"
    );
    assert!(store.inspect(digest(1)).unwrap().unwrap().pending.is_some());
    let result = invalidate_restored_invitations(&db.admin_url, restore).unwrap();
    assert!(result.applied);
    assert_eq!(result.invalidated, 1);
    assert_receipt(&mut db, result);
}

#[test]
fn dump_to_new_database_preserves_accounts_receipts_and_audit_without_duplicate_acceptance() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let pending = issue(&db, &store, 1);
    let pending_mfa = store.save_pending(preparation(&pending, 1)).unwrap();
    let accepted = issue(&db, &store, 2);
    let accepted_mfa = store.save_pending(preparation(&accepted, 2)).unwrap();
    let command = finish(&accepted, &accepted_mfa, 2);
    let mut replay = finish(&accepted, &accepted_mfa, 2);
    replay.operation_id = command.operation_id;
    replay.user = command.user.clone();
    let receipt = store.finish(command).unwrap();
    let mut restore = request(&mut db);
    let original = snapshot(&mut db);
    drop(store);

    let target = backup::RestoredDatabase::copy(&db);
    let mut admin = Client::connect(&target.admin_url, NoTls).unwrap();
    assert!(
        snapshot_client(&mut admin) == original,
        "backup changed invitation inventory"
    );
    restore.expected_database = admin
        .query_one("SELECT current_database()", &[])
        .unwrap()
        .get(0);
    let result = invalidate_restored_invitations(&target.admin_url, restore).unwrap();
    assert!(result.applied);
    assert_eq!(result.invalidated, 1);
    let restored = PostgresInvitationStore::open(&target.runtime_url).unwrap();
    assert!(restored.inspect(digest(1)).unwrap().is_none());
    assert!(restored.finish(finish(&pending, &pending_mfa, 1)).is_err());
    assert!(restored.claim_delivery(claim()).unwrap().is_none());
    assert_eq!(restored.finish(replay).unwrap(), receipt);
    assert_eq!(
        restored
            .completed(digest(2), receipt.operation_id)
            .unwrap()
            .unwrap()
            .receipt,
        receipt,
    );
    assert_eq!(
        restored.get(actor(&db), accepted.id).unwrap().status,
        InvitationStatus::Accepted
    );
    let after = snapshot_client(&mut admin);
    for table in ["users", "operations", "completions"] {
        assert!(
            after[table] == original[table],
            "restored acceptance changed {table}"
        );
    }
    let old_audit = original["audit"].as_array().unwrap();
    let new_audit = after["audit"].as_array().unwrap();
    assert!(
        new_audit[..old_audit.len()] == old_audit[..],
        "restoration changed prior audit"
    );
    assert_eq!(new_audit.len(), old_audit.len() + 1);
    assert_eq!(new_audit.last().unwrap()["action"], RESTORE_ACTION);
    let events = PostgresAuditLog::open(&target.runtime_url)
        .unwrap()
        .load_all()
        .unwrap();
    assert_eq!(events.len(), old_audit.len() + 1);
    assert_eq!(
        verify_chain(&RingSha256Hasher, &events).unwrap(),
        ChainVerification::Valid {
            entries: old_audit.len() + 1,
        }
    );
    assert!(
        snapshot(&mut db) == original,
        "restoration changed the source database"
    );
}
