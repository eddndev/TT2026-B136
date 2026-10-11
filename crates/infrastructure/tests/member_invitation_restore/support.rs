use crate::case_administration_support::Fixture;
use application::{
    identity::invitations::*,
    identity::{Principal, UserRecord},
};
use domain::{
    audit::AuditLog,
    clock::OffsetDateTime,
    crypto::{RecoveryCodeSet, Sha256Digest, RECOVERY_CODE_COUNT},
    identity::{Role, UserId},
};
use infrastructure::{
    identity::{PasswordResetRestoreHead, PasswordResetRestoreRequest, PasswordResetRestoreResult},
    PostgresAuditLog, PostgresInvitationStore,
};

pub const RESTORE_ACTION: &str = "identity.invitation_restore_invalidated";

pub fn fixture() -> Fixture {
    Fixture::new().expect("invitation restoration requires isolated CASE_TEST_DATABASE_URL")
}

pub fn actor(db: &Fixture) -> InvitationActor {
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

pub fn digest(n: u8) -> Sha256Digest {
    Sha256Digest::from_array([n; 32])
}

pub fn issue(db: &Fixture, store: &PostgresInvitationStore, n: u8) -> InvitationSummary {
    store
        .issue(
            actor(db),
            IssueInvitation {
                request: InvitationIssueRequest {
                    operation_id: Uuid::new_v4(),
                    email: format!("invited-{n}@example.test"),
                    role: Role::Litigator,
                },
                token_digest: digest(n),
                protected_token: vec![n; 80],
                at: OffsetDateTime::now_utc(),
            },
        )
        .unwrap()
}

pub fn preparation(s: &InvitationSummary, n: u8) -> PrepareInvitation {
    PrepareInvitation {
        id: s.id,
        revision: s.revision,
        token_digest: digest(n),
        pending: PendingInvitation {
            operation_id: Uuid::new_v4(),
            request_digest: digest(50 + n),
            protected_material: vec![n; 200],
        },
        at: OffsetDateTime::now_utc(),
    }
}

pub fn finish(s: &InvitationSummary, p: &PendingInvitation, n: u8) -> FinishInvitation {
    FinishInvitation {
        id: s.id,
        revision: s.revision,
        pending_operation_id: p.operation_id,
        operation_id: Uuid::new_v4(),
        request_digest: digest(100 + n),
        token_digest: digest(n),
        user: UserRecord {
            id: UserId::new(),
            email: s.email.clone(),
            role: s.role,
            password_hash: "fixture-password".into(),
            active: true,
            protected_totp_secret: vec![n; 80],
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

pub fn claim() -> InvitationDeliveryClaim {
    InvitationDeliveryClaim {
        lease_id: Uuid::new_v4(),
        at: OffsetDateTime::now_utc(),
        lease_seconds: 120,
        provider_window_seconds: 3600,
    }
}

pub fn request(db: &mut Fixture) -> PasswordResetRestoreRequest {
    let events = PostgresAuditLog::open(&db.runtime_url)
        .unwrap()
        .load_all()
        .unwrap();
    PasswordResetRestoreRequest {
        operation_id: Uuid::new_v4(),
        expected_database: db
            .admin
            .query_one("SELECT current_database()", &[])
            .unwrap()
            .get(0),
        expected_schema: db.schema.clone(),
        expected_head: events.last().map(|last| PasswordResetRestoreHead {
            sequence: last.event.sequence,
            chain: last.chain,
        }),
    }
}

pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    snapshot_client(&mut db.admin)
}

pub fn snapshot_client(client: &mut postgres::Client) -> serde_json::Value {
    client.query_one(
        "SELECT jsonb_build_object(
            'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
            'invitations',(SELECT jsonb_agg(to_jsonb(i) ORDER BY id) FROM member_invitations i),
            'operations',(SELECT jsonb_agg(to_jsonb(o) ORDER BY operation_id) FROM member_invitation_operations o),
            'completions',(SELECT jsonb_agg(to_jsonb(c) ORDER BY invitation_id) FROM member_invitation_completions c),
            'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))",
        &[],
    ).unwrap().get(0)
}

pub fn assert_receipt(db: &mut Fixture, result: PasswordResetRestoreResult) {
    let events = PostgresAuditLog::open(&db.runtime_url)
        .unwrap()
        .load_all()
        .unwrap();
    let recorded = &events[usize::try_from(result.audit_sequence).unwrap()];
    assert_eq!(recorded.event.actor, "database-restore");
    assert_eq!(recorded.event.action, RESTORE_ACTION);
    assert_eq!(recorded.chain, result.audit_head);
    assert_eq!(
        events
            .iter()
            .filter(|e| e.event.action == RESTORE_ACTION)
            .count(),
        1
    );
}

pub fn operation_url(db: &Fixture, application: &str) -> String {
    let mut url = reqwest::Url::parse(&db.admin_url).unwrap();
    let parameters: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| key != "options" && key != "application_name")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.set_query(None);
    url.query_pairs_mut()
        .extend_pairs(parameters)
        .append_pair(
            "options",
            &format!("-csearch_path={} -cstatement_timeout=2500", db.schema),
        )
        .append_pair("application_name", application);
    // PostgreSQL URI options need percent-encoded spaces, not form-encoded '+'.
    let query = url.query().unwrap().replace('+', "%20");
    url.set_query(Some(&query));
    url.to_string()
}
