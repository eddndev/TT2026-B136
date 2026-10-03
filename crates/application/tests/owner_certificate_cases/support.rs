#[path = "material.rs"]
mod material;
pub use material::*;

use std::sync::{Arc, Mutex};

use application::{
    credential_trust::CredentialTrustSnapshot,
    identity::{owner_certificates::*, Principal},
    ApplicationError,
};
use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::{CredentialCertificate, CredentialCheck, CredentialFailure, Signature},
    identity::UserId,
    owner_certificates::BindingStatement,
};
use mockall::mock;
use uuid::Uuid;

use crate::case_support::MockIdentity;

mock! { pub Store {} impl OwnerCertificateStore for Store {
    fn load_registration(&self, actor: UserId) -> Result<OwnerRegistrationContext, ApplicationError>;
    fn find(&self, actor: UserId, binding: Uuid) -> Result<Option<OwnerBindingReceipt>, ApplicationError>;
    fn commit_registration(&self, verified: VerifiedOwnerRegistration) -> Result<OwnerBindingCommit, ApplicationError>;
    fn load_withdrawal(&self, actor: UserId, binding: Uuid) -> Result<OwnerWithdrawalContext, ApplicationError>;
    fn commit_withdrawal(&self, prepared: PreparedOwnerWithdrawal) -> Result<OwnerBindingCommit, ApplicationError>;
} }

mock! { pub Verifier {} impl OwnerBindingVerifier for Verifier {
    fn inspect_certificate(&self, certificate: &[u8]) -> Result<CredentialCertificate, CredentialFailure>;
    fn verify_registration(&self, statement: &BindingStatement, certificate: &[u8], signature: &Signature,
        trust: &CredentialTrustSnapshot, at: i64) -> Result<CredentialCheck, OwnerCertificateError>;
} }

pub struct State {
    pub principal: Option<Principal>,
    pub context: OwnerRegistrationContext,
    pub found: Option<OwnerBindingReceipt>,
    pub calls: Vec<&'static str>,
    pub now: i64,
    pub expire_on_find: bool,
}

pub struct TestClock(Arc<Mutex<State>>);
impl Clock for TestClock {
    fn now(&self) -> OffsetDateTime {
        at(self.0.lock().unwrap().now)
    }
}

pub struct Harness {
    pub state: Arc<Mutex<State>>,
    pub identity: MockIdentity,
    pub store: MockStore,
    pub verifier: MockVerifier,
}

impl Harness {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(State {
            principal: Some(principal()),
            context: context(),
            found: None,
            calls: Vec::new(),
            now: 1000,
            expire_on_find: false,
        }));
        let mut identity = MockIdentity::new();
        let observed = state.clone();
        identity.expect_authenticate().returning(move |token| {
            assert_eq!(token, "session");
            let mut state = observed.lock().unwrap();
            state.calls.push("authenticate");
            state
                .principal
                .clone()
                .ok_or(ApplicationError::InvalidSession)
        });
        let mut store = MockStore::new();
        let observed = state.clone();
        store.expect_load_registration().returning(move |actor| {
            assert_eq!(actor, principal().id);
            let mut state = observed.lock().unwrap();
            state.calls.push("load_registration");
            Ok(state.context.clone())
        });
        let observed = state.clone();
        store.expect_find().returning(move |actor, id| {
            assert_eq!((actor, id), (principal().id, binding()));
            let mut state = observed.lock().unwrap();
            state.calls.push("find");
            if state.expire_on_find {
                state.principal = None;
            }
            Ok(state.found.clone())
        });
        let mut verifier = MockVerifier::new();
        let observed = state.clone();
        verifier
            .expect_inspect_certificate()
            .returning(move |bytes| {
                assert_eq!(bytes, RAW_CERTIFICATE);
                observed.lock().unwrap().calls.push("inspect");
                Ok(certificate())
            });
        Self {
            state,
            identity,
            store,
            verifier,
        }
    }

    pub fn verify_with(
        &mut self,
        effect: impl Fn(&mut CredentialCheck, &Arc<Mutex<State>>) + Send + Sync + 'static,
    ) {
        let observed = self.state.clone();
        self.verifier
            .expect_verify_registration()
            .times(1)
            .returning(move |intent, certificate_bytes, signature, supplied, now| {
                assert_eq!(intent, &statement());
                assert_eq!(certificate_bytes, certificate().der);
                assert_eq!(supplied, &trust());
                observed.lock().unwrap().calls.push("verify");
                let mut result = check(intent, signature, supplied, now);
                effect(&mut result, &observed);
                Ok(result)
            });
    }

    pub fn withdrawal(&mut self, value: OwnerBindingReceipt) {
        let observed = self.state.clone();
        self.store
            .expect_load_withdrawal()
            .times(1)
            .returning(move |actor, id| {
                assert_eq!((actor, id), (principal().id, binding()));
                let mut state = observed.lock().unwrap();
                state.calls.push("load_withdrawal");
                Ok(OwnerWithdrawalContext {
                    account: state.context.account.clone(),
                    receipt: value.clone(),
                })
            });
    }

    pub fn service(self) -> OwnerCertificateService {
        OwnerCertificateService::new(OwnerCertificatePorts {
            identity: Arc::new(self.identity),
            store: Arc::new(self.store),
            verifier: Arc::new(self.verifier),
            hasher: Arc::new(Hasher),
            clock: Arc::new(TestClock(self.state)),
        })
    }
}

pub fn prepare(service: &OwnerCertificateService) -> PreparedOwnerRegistration {
    service
        .prepare_registration("session", binding(), RAW_CERTIFICATE)
        .unwrap()
}

pub fn kind(error: &ApplicationError) -> &OwnerCertificateError {
    match error {
        ApplicationError::OwnerCertificate(value) => value,
        _ => panic!("expected an owner certificate error category"),
    }
}

pub fn failure<T>(result: Result<T, ApplicationError>) -> ApplicationError {
    match result {
        Err(error) => error,
        Ok(_) => panic!("expected the owner certificate operation to reject"),
    }
}
