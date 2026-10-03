use std::sync::{Arc, Mutex};

use application::{identity::*, ApplicationError};
use domain::identity::{Permission, Role, UserId};
use uuid::Uuid;

use crate::support::*;

/// Authentication is a deliberate double; persistence and RSA remain real.
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

/// Captures opaque commands produced by the real service, never manufactures one.
struct Capture {
    inner: Arc<PostgresOwnerCertificateStore>,
    registration: Mutex<Option<VerifiedOwnerRegistration>>,
    withdrawal: Mutex<Option<PreparedOwnerWithdrawal>>,
}
impl Capture {
    fn new(inner: Arc<PostgresOwnerCertificateStore>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            registration: Mutex::new(None),
            withdrawal: Mutex::new(None),
        })
    }
}
impl OwnerCertificateStore for Capture {
    fn load_registration(
        &self,
        actor: UserId,
    ) -> Result<OwnerRegistrationContext, ApplicationError> {
        self.inner.load_registration(actor)
    }
    fn find(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<Option<OwnerBindingReceipt>, ApplicationError> {
        self.inner.find(actor, binding)
    }
    fn find_current(&self, actor: UserId) -> Result<Option<OwnerBindingReceipt>, ApplicationError> {
        self.inner.find_current(actor)
    }
    fn load_withdrawal(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<OwnerWithdrawalContext, ApplicationError> {
        self.inner.load_withdrawal(actor, binding)
    }
    fn commit_registration(
        &self,
        command: VerifiedOwnerRegistration,
    ) -> Result<OwnerBindingCommit, ApplicationError> {
        assert!(self.registration.lock().unwrap().replace(command).is_none());
        Err(ApplicationError::Port(
            "captured registration command".into(),
        ))
    }
    fn commit_withdrawal(
        &self,
        command: PreparedOwnerWithdrawal,
    ) -> Result<OwnerBindingCommit, ApplicationError> {
        assert!(self.withdrawal.lock().unwrap().replace(command).is_none());
        Err(ApplicationError::Port("captured withdrawal command".into()))
    }
}

pub fn registration(
    store: Arc<PostgresOwnerCertificateStore>,
    clock: &Arc<Clock>,
    actor: Principal,
    id: Uuid,
) -> VerifiedOwnerRegistration {
    let capture = Capture::new(store);
    let service = service(capture.clone(), clock, actor);
    let (prepared, signature) = preparation(&service, id);
    assert!(matches!(
        service.register(TOKEN, &prepared, &signature),
        Err(ApplicationError::Port(_))
    ));
    let mut command = capture.registration.lock().unwrap();
    command
        .take()
        .expect("service must reach the registration commit")
}

pub fn withdrawal(
    store: Arc<PostgresOwnerCertificateStore>,
    clock: &Arc<Clock>,
    actor: Principal,
    id: Uuid,
) -> PreparedOwnerWithdrawal {
    let capture = Capture::new(store);
    let service = service(capture.clone(), clock, actor);
    assert!(matches!(
        service.withdraw(TOKEN, id, 1),
        Err(ApplicationError::Port(_))
    ));
    let mut command = capture.withdrawal.lock().unwrap();
    command
        .take()
        .expect("service must reach the withdrawal commit")
}
