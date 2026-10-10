mod material;
pub use material::*;

use crate::case_support::MockIdentity;
use application::{
    credential_trust::CredentialTrustSnapshot, document_signing::*, identity::Principal,
    ApplicationError,
};
use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::{CredentialCertificate, CredentialCheck, CredentialFailure, Signature},
    document_signing::{DocumentConsent, SignerAccount, SigningBindingStatement},
    identity::UserId,
};
use mockall::mock;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

mock! { pub Store {} impl SigningBindingStore for Store {
    fn load_context(&self, actor: UserId) -> Result<SigningAccountContext, ApplicationError>;
    fn find(&self, actor: UserId, binding: Uuid) -> Result<Option<SigningBindingReceipt>, ApplicationError>;
    fn find_current(&self, actor: UserId) -> Result<Option<SigningBindingReceipt>, ApplicationError>;
    fn commit_binding(&self, verified: VerifiedSigningBinding) -> Result<SigningBindingReceipt, ApplicationError>;
    fn withdraw(&self, principal: &Principal, binding: Uuid, expected: SignerAccount, at: OffsetDateTime) -> Result<SigningBindingReceipt, ApplicationError>;
} }
mock! { pub Verifier {} impl DocumentSigningVerifier for Verifier {
    fn inspect_certificate(&self, certificate: &[u8]) -> Result<CredentialCertificate, CredentialFailure>;
    fn verify_binding(&self, statement: &SigningBindingStatement, certificate: &[u8], signature: &Signature,
        trust: &CredentialTrustSnapshot, at: i64) -> Result<CredentialCheck, SigningWorkflowError>;
    fn verify_consent(&self, consent: &DocumentConsent, certificate: &[u8], pdf_signature: &Signature,
        consent_signature: &Signature, trust: &CredentialTrustSnapshot, at: i64) -> Result<CredentialCheck, SigningWorkflowError>;
} }

pub struct State {
    pub principal: Option<Principal>,
    pub context: SigningAccountContext,
    pub found: Option<SigningBindingReceipt>,
    pub calls: Vec<&'static str>,
    pub now: i64,
    pub change_on_find: bool,
}
struct TestClock(Arc<Mutex<State>>);
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
            change_on_find: false,
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
        store.expect_load_context().returning(move |actor| {
            assert_eq!(actor, principal().id);
            let mut state = observed.lock().unwrap();
            state.calls.push("context");
            Ok(state.context.clone())
        });
        let observed = state.clone();
        store.expect_find().returning(move |actor, binding| {
            assert_eq!((actor, binding), (principal().id, id()));
            let mut state = observed.lock().unwrap();
            state.calls.push("find");
            if state.change_on_find {
                state.principal.as_mut().unwrap().email = "changed@example.test".into();
            }
            Ok(state.found.clone())
        });
        let observed = state.clone();
        store.expect_find_current().returning(move |actor| {
            assert_eq!(actor, principal().id);
            let mut state = observed.lock().unwrap();
            state.calls.push("current");
            if state.change_on_find {
                state.principal.as_mut().unwrap().email = "changed@example.test".into();
            }
            Ok(state.found.clone())
        });
        let mut verifier = MockVerifier::new();
        let observed = state.clone();
        verifier
            .expect_inspect_certificate()
            .returning(move |bytes| {
                assert_eq!(bytes, certificate().der);
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
        self.verifier.expect_verify_binding().times(1).returning(
            move |intent, cert, signature, trust, now| {
                assert_eq!(intent, &statement());
                assert_eq!(cert, certificate().der);
                observed.lock().unwrap().calls.push("verify");
                let mut value = check(intent, signature, trust, now);
                effect(&mut value, &observed);
                Ok(value)
            },
        );
    }
    pub fn service(self) -> SigningBindingService {
        SigningBindingService::new(SigningBindingPorts {
            identity: Arc::new(self.identity),
            store: Arc::new(self.store),
            verifier: Arc::new(self.verifier),
            hasher: Arc::new(Hasher),
            clock: Arc::new(TestClock(self.state)),
        })
    }
}
pub fn failure<T>(result: Result<T, ApplicationError>) -> ApplicationError {
    match result {
        Err(error) => error,
        Ok(_) => panic!("operation should reject"),
    }
}
pub fn kind(error: ApplicationError) -> SigningWorkflowError {
    match error {
        ApplicationError::DocumentSigning(value) => value,
        other => panic!("expected document signing error, got {other}"),
    }
}
