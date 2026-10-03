use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

pub use application::identity::owner_certificates::*;
use application::{
    credential_trust::{CredentialTrustExpectation, CredentialTrustSnapshot, CredentialTrustStore},
    identity::Principal,
    ApplicationError,
};
use domain::{
    audit::{AuditLog, ChainVerification},
    clock::OffsetDateTime,
    crypto::Signature,
    identity::{Role, UserId},
};
pub use infrastructure::PostgresOwnerCertificateStore;
use infrastructure::{
    certificates::InternalRsaOwnerBindingVerifier, PostgresAuditLog, PostgresCredentialTrustStore,
    RingSha256Hasher,
};
use uuid::Uuid;

pub use crate::case_administration_support::Fixture;
use crate::{capture::FixedIdentity, owner_binding_fixture};

pub const REGISTER: &str = "identity.owner_certificate_registered";
pub const WITHDRAW: &str = "identity.owner_certificate_withdrawn";
pub const TOKEN: &str = "synthetic-session";

pub struct Clock(AtomicI64);
impl Clock {
    pub fn new(at: i64) -> Arc<Self> {
        Arc::new(Self(AtomicI64::new(at)))
    }
    pub fn set(&self, at: i64) {
        self.0.store(at, Ordering::SeqCst);
    }
    pub fn at(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.0.load(Ordering::SeqCst))
            .unwrap()
            .replace_nanosecond(123_456_789)
            .unwrap()
    }
}
impl domain::clock::Clock for Clock {
    fn now(&self) -> OffsetDateTime {
        self.at()
    }
}

pub fn fixture() -> Option<(Fixture, Arc<Clock>, CredentialTrustSnapshot)> {
    let db = Fixture::new()?;
    let material = owner_binding_fixture::fixture();
    let clock = Clock::new(material.at);
    let trust = PostgresCredentialTrustStore::open(&db.admin_url, clock.clone())
        .unwrap()
        .publish(
            CredentialTrustExpectation::Absent,
            material.trust.inspection.clone(),
        )
        .unwrap();
    Some((db, clock, trust))
}

pub fn store(db: &Fixture, clock: &Arc<Clock>) -> Arc<PostgresOwnerCertificateStore> {
    Arc::new(PostgresOwnerCertificateStore::open(&db.runtime_url, clock.clone()).unwrap())
}

pub fn principal(db: &mut Fixture, id: UserId) -> Principal {
    let row = db
        .admin
        .query_one("SELECT email,role FROM users WHERE id=$1", &[&id.as_uuid()])
        .unwrap();
    let role: String = row.get(1);
    Principal {
        id,
        email: row.get(0),
        role: match role.as_str() {
            "owner" => Role::Owner,
            "litigator" => Role::Litigator,
            "paralegal" => Role::Paralegal,
            "client" => Role::Client,
            _ => panic!("unknown fixture role"),
        },
    }
}

pub fn service(
    store: Arc<dyn OwnerCertificateStore>,
    clock: &Arc<Clock>,
    actor: Principal,
) -> OwnerCertificateService {
    OwnerCertificateService::new(OwnerCertificatePorts {
        identity: Arc::new(FixedIdentity(actor)),
        store,
        verifier: Arc::new(InternalRsaOwnerBindingVerifier::new()),
        hasher: Arc::new(RingSha256Hasher),
        clock: clock.clone(),
    })
}

pub fn preparation(
    service: &OwnerCertificateService,
    id: Uuid,
) -> (PreparedOwnerRegistration, Signature) {
    let prepared = service
        .prepare_registration(TOKEN, id, &owner_binding_fixture::fixture().leaf)
        .unwrap();
    let signature = owner_binding_fixture::sign(&prepared.statement().canonical_bytes());
    (prepared, signature)
}

pub fn applied(value: OwnerBindingCommit) -> OwnerBindingReceipt {
    match value {
        OwnerBindingCommit::Applied(receipt) => receipt,
        OwnerBindingCommit::Existing(_) => panic!("expected a new owner certificate mutation"),
    }
}

pub fn existing(value: OwnerBindingCommit) -> OwnerBindingReceipt {
    match value {
        OwnerBindingCommit::Existing(receipt) => receipt,
        OwnerBindingCommit::Applied(_) => panic!("expected an exact existing receipt"),
    }
}

pub fn category<T>(result: Result<T, ApplicationError>, expected: OwnerCertificateError) {
    match result {
        Err(ApplicationError::OwnerCertificate(error)) => assert_eq!(error, expected),
        _ => panic!("expected the neutral owner certificate rejection category"),
    }
}

pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
        'registrations',(SELECT jsonb_agg(to_jsonb(r) ORDER BY binding_id) FROM owner_certificate_registrations r),
        'withdrawals',(SELECT jsonb_agg(to_jsonb(w) ORDER BY binding_id) FROM owner_certificate_withdrawals w),
        'trust',(SELECT jsonb_agg(to_jsonb(t) ORDER BY revision) FROM participant_credential_trust_revisions t),
        'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
}

pub fn users(db: &mut Fixture) -> serde_json::Value {
    db.admin
        .query_one(
            "SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u",
            &[],
        )
        .unwrap()
        .get(0)
}

pub fn counts(db: &mut Fixture) -> (i64, i64, i64) {
    let row = db
        .admin
        .query_one(
            "SELECT
        (SELECT count(*) FROM owner_certificate_registrations),
        (SELECT count(*) FROM owner_certificate_withdrawals),
        (SELECT count(*) FROM audit_events WHERE action IN ($1,$2))",
            &[&REGISTER, &WITHDRAW],
        )
        .unwrap();
    (row.get(0), row.get(1), row.get(2))
}

pub fn assert_audit(db: &Fixture, receipts: &[(&OwnerBindingReceipt, bool)]) {
    use domain::crypto::DocumentHasher;
    let events = PostgresAuditLog::open(&db.runtime_url)
        .unwrap()
        .load_all()
        .unwrap();
    assert_eq!(
        domain::audit::verify_chain(&RingSha256Hasher, &events).unwrap(),
        ChainVerification::Valid {
            entries: events.len()
        }
    );
    let mutations: Vec<_> = events
        .iter()
        .filter(|row| [REGISTER, WITHDRAW].contains(&row.event.action.as_str()))
        .collect();
    assert_eq!(mutations.len(), receipts.len());
    for (row, (receipt, withdrawal)) in mutations.into_iter().zip(receipts) {
        let bytes = if *withdrawal {
            receipt.record.withdrawal().unwrap().canonical_bytes()
        } else {
            receipt.record.registration().canonical_bytes()
        };
        let revision = if *withdrawal { 2 } else { 1 };
        assert_eq!(
            row.event.action,
            if *withdrawal { WITHDRAW } else { REGISTER }
        );
        assert_eq!(row.event.actor, "owner@example.test");
        assert_eq!(
            row.event.timestamp,
            if *withdrawal {
                receipt.withdrawn_at.unwrap()
            } else {
                receipt.registered_at
            }
        );
        assert_eq!(
            row.event.resource,
            format!(
                "owner-certificate:{}:owner:{}:revision:{revision}:statement:{}",
                receipt.record.registration().material().binding(),
                receipt.owner,
                RingSha256Hasher.hash_bytes(&bytes).to_hex()
            )
        );
    }
}
