use super::{Clock, PostgresSigningBindingStore, TOKEN};
use application::{document_signing::*, identity::Principal, ApplicationError};
use domain::{clock::OffsetDateTime, document_signing::SignerAccount, identity::UserId};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

struct Capture {
    inner: Arc<PostgresSigningBindingStore>,
    captured: Mutex<Option<VerifiedSigningBinding>>,
}
impl SigningBindingStore for Capture {
    fn load_context(&self, actor: UserId) -> Result<SigningAccountContext, ApplicationError> {
        self.inner.load_context(actor)
    }
    fn find(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<Option<SigningBindingReceipt>, ApplicationError> {
        self.inner.find(actor, binding)
    }
    fn find_current(
        &self,
        actor: UserId,
    ) -> Result<Option<SigningBindingReceipt>, ApplicationError> {
        self.inner.find_current(actor)
    }
    fn commit_binding(
        &self,
        command: VerifiedSigningBinding,
    ) -> Result<SigningBindingReceipt, ApplicationError> {
        assert!(self.captured.lock().unwrap().replace(command).is_none());
        Err(ApplicationError::Port("captured signing command".into()))
    }
    fn withdraw(
        &self,
        _: &Principal,
        _: Uuid,
        _: SignerAccount,
        _: OffsetDateTime,
    ) -> Result<SigningBindingReceipt, ApplicationError> {
        unreachable!()
    }
}
pub fn command(
    store: Arc<PostgresSigningBindingStore>,
    clock: &Arc<Clock>,
    actor: Principal,
    id: Uuid,
) -> VerifiedSigningBinding {
    let capture = Arc::new(Capture {
        inner: store,
        captured: Mutex::new(None),
    });
    let service = super::service(capture.clone(), clock, actor);
    let (prepared, signature) = super::prepare(&service, id);
    assert!(matches!(
        service.register(
            TOKEN,
            prepared.statement(),
            &prepared.certificate().der,
            &signature
        ),
        Err(ApplicationError::Port(_))
    ));
    let value = capture.captured.lock().unwrap().take().unwrap();
    value
}
