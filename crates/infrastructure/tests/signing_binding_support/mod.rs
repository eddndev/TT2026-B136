mod capture;
pub use crate::case_administration_support::Fixture;
pub use application::document_signing::*;
use application::{
    credential_trust::{CredentialTrustExpectation, CredentialTrustStore},
    identity::*,
    ApplicationError,
};
#[allow(unused_imports)]
pub use capture::command;
use domain::{
    clock::{Clock as ClockPort, OffsetDateTime},
    crypto::Signature,
    identity::{Permission, Role, UserId},
};
pub use infrastructure::PostgresSigningBindingStore;
use infrastructure::{
    certificates::InternalRsaDocumentSigningVerifier, PostgresCredentialTrustStore,
    RingSha256Hasher,
};
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};
use uuid::Uuid;

pub const TOKEN: &str = "synthetic-session";
pub struct Clock(AtomicI64);
impl Clock {
    pub fn set(&self, at: i64) {
        self.0.store(at, Ordering::SeqCst);
    }
}
impl ClockPort for Clock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.0.load(Ordering::SeqCst)).unwrap()
    }
}
pub fn fixture() -> Option<(Fixture, Arc<Clock>, Principal)> {
    let mut db = Fixture::new()?;
    let clock = Arc::new(Clock(AtomicI64::new(
        crate::declaration_fixture::fixture().at,
    )));
    PostgresCredentialTrustStore::open(&db.admin_url, clock.clone())
        .unwrap()
        .publish(
            CredentialTrustExpectation::Absent,
            crate::individual_signing_fixture::trust().inspection,
        )
        .unwrap();
    let user = db.user("litigator", false);
    let actor = principal(&mut db, user);
    Some((db, clock, actor))
}
pub fn principal(db: &mut Fixture, id: UserId) -> Principal {
    let row = db
        .admin
        .query_one("SELECT email,role FROM users WHERE id=$1", &[&id.as_uuid()])
        .unwrap();
    Principal {
        id,
        email: row.get(0),
        role: row.get::<_, String>(1).parse().unwrap(),
    }
}
pub fn store(db: &Fixture, clock: &Arc<Clock>) -> Arc<PostgresSigningBindingStore> {
    Arc::new(PostgresSigningBindingStore::open(&db.runtime_url, clock.clone()).unwrap())
}
pub fn service(
    store: Arc<dyn SigningBindingStore>,
    clock: &Arc<Clock>,
    principal: Principal,
) -> SigningBindingService {
    SigningBindingService::new(SigningBindingPorts {
        identity: Arc::new(FixedIdentity(principal)),
        store,
        verifier: Arc::new(InternalRsaDocumentSigningVerifier),
        hasher: Arc::new(RingSha256Hasher),
        clock: clock.clone(),
    })
}
pub fn prepare(service: &SigningBindingService, id: Uuid) -> (PreparedSigningBinding, Signature) {
    let prepared = service
        .prepare(TOKEN, id, &crate::declaration_fixture::fixture().leaf)
        .unwrap();
    let signature =
        crate::individual_signing_fixture::sign(&prepared.statement().canonical_bytes());
    (prepared, signature)
}
pub fn register(service: &SigningBindingService, id: Uuid) -> SigningBindingReceipt {
    let (prepared, signature) = prepare(service, id);
    service
        .register(
            TOKEN,
            prepared.statement(),
            &prepared.certificate().der,
            &signature,
        )
        .unwrap()
}
pub fn counts(db: &mut Fixture) -> (i64, i64, i64) {
    let row = db.admin.query_one("SELECT (SELECT count(*) FROM document_signing_bindings),
        (SELECT count(*) FROM document_signing_withdrawals),
        (SELECT count(*) FROM audit_events WHERE action IN ('document.signer_bound','document.signer_withdrawn'))", &[]).unwrap();
    (row.get(0), row.get(1), row.get(2))
}
pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'bindings',(SELECT jsonb_agg(to_jsonb(r) ORDER BY binding_id) FROM document_signing_bindings r),
        'withdrawals',(SELECT jsonb_agg(to_jsonb(w) ORDER BY binding_id) FROM document_signing_withdrawals w),
        'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
}
pub fn category<T>(result: Result<T, ApplicationError>, expected: SigningWorkflowError) {
    match result {
        Err(ApplicationError::DocumentSigning(error)) => assert_eq!(error, expected),
        _ => panic!("expected signing category {expected}"),
    }
}
pub struct FixedIdentity(pub Principal);
impl IdentityWorkflow for FixedIdentity {
    fn authenticate(&self, token: &str) -> Result<Principal, ApplicationError> {
        assert_eq!(token, TOKEN);
        Ok(self.0.clone())
    }
    fn session_status(&self, _: &str) -> Result<SessionStatus, ApplicationError> {
        unreachable!()
    }
    fn record_activity(&self, _: &str) -> Result<SessionStatus, ApplicationError> {
        unreachable!()
    }
    fn authorize(&self, _: &str, _: Permission) -> Result<Principal, ApplicationError> {
        unreachable!()
    }
    fn bootstrap_owner(&self, _: &str, _: &str) -> Result<EnrollmentResult, ApplicationError> {
        unreachable!()
    }
    fn create_user(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: Role,
    ) -> Result<EnrollmentResult, ApplicationError> {
        unreachable!()
    }
    fn start_login(&self, _: &str, _: &str) -> Result<LoginChallenge, ApplicationError> {
        unreachable!()
    }
    fn complete_totp(&self, _: &str, _: &str) -> Result<SessionResult, ApplicationError> {
        unreachable!()
    }
    fn complete_recovery(&self, _: &str, _: &str) -> Result<SessionResult, ApplicationError> {
        unreachable!()
    }
    fn logout(&self, _: &str) -> Result<(), ApplicationError> {
        unreachable!()
    }
}
