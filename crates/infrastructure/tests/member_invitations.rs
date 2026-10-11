#[path = "case_administration_support/mod.rs"]
mod case_administration_support;

use application::{
    identity::invitations::*,
    identity::{Principal, UserRecord},
    ApplicationError,
};
use case_administration_support::Fixture;
use domain::{
    clock::OffsetDateTime,
    crypto::{RecoveryCodeSet, Sha256Digest, RECOVERY_CODE_COUNT},
    identity::{Role, UserId},
};
use infrastructure::PostgresInvitationStore;

fn fixture() -> Fixture {
    Fixture::new().expect("invitation acceptance requires isolated CASE_TEST_DATABASE_URL")
}
fn actor(db: &Fixture) -> InvitationActor {
    InvitationActor {
        principal: Principal {
            id: db.owner,
            email: "owner@example.test".into(),
            role: Role::Owner,
        },
        revision: 0,
        auth_generation: 0,
    }
}
fn digest(n: u8) -> Sha256Digest {
    Sha256Digest::from_array([n; 32])
}
fn issue(op: Uuid, email: &str) -> IssueInvitation {
    IssueInvitation {
        request: InvitationIssueRequest {
            operation_id: op,
            email: email.into(),
            role: Role::Litigator,
        },
        token_digest: digest(1),
        protected_token: vec![3; 80],
        at: OffsetDateTime::now_utc(),
    }
}
fn pending() -> PendingInvitation {
    PendingInvitation {
        operation_id: Uuid::new_v4(),
        request_digest: digest(3),
        protected_material: vec![7; 200],
    }
}
fn prepared(id: InvitationId, pending: PendingInvitation) -> PrepareInvitation {
    PrepareInvitation {
        id,
        revision: 1,
        token_digest: digest(1),
        pending,
        at: OffsetDateTime::now_utc(),
    }
}
fn finish(id: InvitationId, pending: &PendingInvitation) -> FinishInvitation {
    FinishInvitation {
        id,
        revision: 1,
        pending_operation_id: pending.operation_id,
        operation_id: Uuid::new_v4(),
        request_digest: digest(4),
        token_digest: digest(1),
        user: UserRecord {
            id: UserId::new(),
            email: "invited@example.test".into(),
            role: Role::Litigator,
            password_hash: "fixture-password".into(),
            active: true,
            protected_totp_secret: vec![5; 80],
            recovery_codes: RecoveryCodeSet::from_hashes(
                (0..RECOVERY_CODE_COUNT)
                    .map(|n| format!("recovery-{n}"))
                    .collect(),
            )
            .unwrap(),
            revision: 0,
            auth_generation: 0,
        },
        at: OffsetDateTime::now_utc(),
    }
}

#[test]
fn acceptance_preserves_pending_material_and_commits_one_account_with_receipt() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let op = Uuid::new_v4();
    let first = store
        .issue(actor(&db), issue(op, "invited@example.test"))
        .unwrap();
    let replay = store
        .issue(actor(&db), issue(op, "invited@example.test"))
        .unwrap();
    assert_eq!(first, replay);
    assert!(matches!(
        store.issue(actor(&db), issue(op, "different@example.test")),
        Err(ApplicationError::ConcurrentModification)
    ));
    let p = store.save_pending(prepared(first.id, pending())).unwrap();
    let restored = store.save_pending(prepared(first.id, pending())).unwrap();
    assert_eq!(p.protected_material, restored.protected_material);
    assert_eq!(p.operation_id, restored.operation_id);
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM users WHERE email='invited@example.test'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 0, "pending MFA must not create a login account");
    let command = finish(first.id, &p);
    let completed_op = command.operation_id;
    let receipt = store.finish(command).unwrap();
    assert!(store.inspect(digest(1)).unwrap().is_none());
    let repeated = store.completed(digest(1), completed_op).unwrap().unwrap();
    assert_eq!(repeated.receipt, receipt);
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM users WHERE email='invited@example.test'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
    let state = store.get(actor(&db), first.id).unwrap();
    assert_eq!(state.status, InvitationStatus::Accepted);
    let remaining:bool=db.admin.query_one("SELECT pending_material IS NULL AND protected_token IS NULL FROM member_invitations WHERE id=$1",&[&first.id.as_uuid()]).unwrap().get(0);
    assert!(
        remaining,
        "accepted invitation must erase recoverable secrets"
    );
}

#[test]
fn resend_rejects_old_link_and_old_pending_without_creating_an_account() {
    let db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let first = store
        .issue(actor(&db), issue(Uuid::new_v4(), "invited@example.test"))
        .unwrap();
    let p = store.save_pending(prepared(first.id, pending())).unwrap();
    let operation = Uuid::new_v4();
    let resend = || ResendInvitation {
        request: InvitationResendRequest {
            operation_id: operation,
            expected_revision: 1,
        },
        token_digest: digest(2),
        protected_token: vec![8; 80],
        at: OffsetDateTime::now_utc(),
    };
    let next = store.resend(actor(&db), first.id, resend()).unwrap();
    assert_eq!(next.revision, 2);
    assert_eq!(next.status, InvitationStatus::Pending);
    assert_eq!(store.resend(actor(&db), first.id, resend()).unwrap(), next);
    assert!(store.inspect(digest(1)).unwrap().is_none());
    assert!(store.inspect(digest(2)).unwrap().unwrap().pending.is_none());
    assert!(store.finish(finish(first.id, &p)).is_err());
}

#[test]
fn existing_accounts_and_revoked_owner_never_issue() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    assert!(matches!(
        store.issue(actor(&db), issue(Uuid::new_v4(), "owner@example.test")),
        Err(ApplicationError::UserAlreadyExists)
    ));
    let observed = actor(&db);
    db.admin
        .execute(
            "UPDATE users SET revision=revision+1 WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        store.issue(observed, issue(Uuid::new_v4(), "invited@example.test")),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn rejected_audit_rolls_back_account_capability_and_receipt() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let first = store
        .issue(actor(&db), issue(Uuid::new_v4(), "invited@example.test"))
        .unwrap();
    let p = store.save_pending(prepared(first.id, pending())).unwrap();
    db.admin.batch_execute("CREATE FUNCTION reject_invitation_audit() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN IF NEW.action='identity.invitation_accepted' THEN RAISE EXCEPTION 'fixture'; END IF; RETURN NEW; END$$; CREATE TRIGGER reject_invitation_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_invitation_audit()").unwrap();
    assert!(store.finish(finish(first.id, &p)).is_err());
    assert!(store.inspect(digest(1)).unwrap().unwrap().pending.is_some());
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM users WHERE email='invited@example.test'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
}

#[test]
fn delivery_retry_keeps_provider_identity_and_resend_fences_old_worker() {
    let db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let first = store
        .issue(actor(&db), issue(Uuid::new_v4(), "invited@example.test"))
        .unwrap();
    let claim = || InvitationDeliveryClaim {
        lease_id: Uuid::new_v4(),
        at: OffsetDateTime::now_utc(),
        lease_seconds: 30,
        provider_window_seconds: 3600,
    };
    let lease = store.claim_delivery(claim()).unwrap().unwrap();
    assert!(store.claim_delivery(claim()).unwrap().is_none());
    assert!(store
        .finish_delivery(InvitationDeliveryFinish {
            id: lease.id,
            revision: lease.revision,
            generation: lease.generation,
            lease_id: lease.lease_id,
            state: InvitationDeliveryState::Uncertain,
            retry_at: Some(OffsetDateTime::now_utc()),
            at: OffsetDateTime::now_utc()
        })
        .unwrap());
    let second = store.claim_delivery(claim()).unwrap().unwrap();
    assert_eq!(second.delivery_id, lease.delivery_id);
    assert_eq!(second.provider_deadline, lease.provider_deadline);
    assert!(second.generation > lease.generation);
    store
        .resend(
            actor(&db),
            first.id,
            ResendInvitation {
                request: InvitationResendRequest {
                    operation_id: Uuid::new_v4(),
                    expected_revision: 1,
                },
                token_digest: digest(2),
                protected_token: vec![8; 80],
                at: OffsetDateTime::now_utc(),
            },
        )
        .unwrap();
    assert!(!store
        .finish_delivery(InvitationDeliveryFinish {
            id: second.id,
            revision: second.revision,
            generation: second.generation,
            lease_id: second.lease_id,
            state: InvitationDeliveryState::Sent,
            retry_at: None,
            at: OffsetDateTime::now_utc()
        })
        .unwrap());
    let third = store.claim_delivery(claim()).unwrap().unwrap();
    assert_ne!(third.delivery_id, lease.delivery_id);
    assert_eq!(third.revision, 2);
}

#[test]
fn expired_capability_never_prepares_or_delivers_and_requires_explicit_resend() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let first = store
        .issue(actor(&db), issue(Uuid::new_v4(), "invited@example.test"))
        .unwrap();
    db.admin
        .execute(
            "UPDATE member_invitations SET issued_at=0,expires_at=1 WHERE id=$1",
            &[&first.id.as_uuid()],
        )
        .unwrap();
    assert!(store.inspect(digest(1)).unwrap().is_none());
    assert!(store.save_pending(prepared(first.id, pending())).is_err());
    assert!(store
        .claim_delivery(InvitationDeliveryClaim {
            lease_id: Uuid::new_v4(),
            at: OffsetDateTime::now_utc(),
            lease_seconds: 30,
            provider_window_seconds: 3600
        })
        .unwrap()
        .is_none());
    assert_eq!(
        store.get(actor(&db), first.id).unwrap().status,
        InvitationStatus::Expired
    );
}

#[test]
fn concurrent_completion_reconciles_to_one_account_and_one_audit_event() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let first = store
        .issue(actor(&db), issue(Uuid::new_v4(), "invited@example.test"))
        .unwrap();
    let p = store.save_pending(prepared(first.id, pending())).unwrap();
    let c1 = finish(first.id, &p);
    let mut c2 = finish(first.id, &p);
    c2.operation_id = c1.operation_id;
    c2.user = c1.user.clone();
    let store2 = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other = barrier.clone();
    let worker = std::thread::spawn(move || {
        other.wait();
        store2.finish(c2).unwrap()
    });
    barrier.wait();
    let receipt = store.finish(c1).unwrap();
    assert_eq!(worker.join().unwrap(), receipt);
    let counts=db.admin.query_one("SELECT (SELECT count(*) FROM users WHERE email='invited@example.test'),(SELECT count(*) FROM audit_events WHERE action='identity.invitation_accepted')",&[]).unwrap();
    assert_eq!(counts.get::<_, i64>(0), 1);
    assert_eq!(counts.get::<_, i64>(1), 1);
}
