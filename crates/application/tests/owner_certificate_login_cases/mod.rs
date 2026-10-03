use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub use application::{
    identity::{
        certificate_login::*, IdentityPorts, IdentityService, LoginChallenge,
        LoginChallengeIdentity, Principal, SessionGrant, SessionIdentity, SessionPolicy,
        SessionResult, SessionState, SessionStore, UserRecord,
    },
    ApplicationError,
};
use domain::{audit::ChainedEvent, crypto::DocumentHasher};

mod certificate_ports;
mod identity_ports;
mod material;
mod sessions;
pub use material::{at, binding, context, user, Hasher};

#[derive(Clone, Copy, Debug)]
pub enum Change {
    Trust,
    Withdraw,
    Principal,
    Generation,
    Advance(i64),
}

#[derive(Clone, Copy, Debug)]
pub enum CheckFault {
    Digest,
    Signature,
    Certificate,
    Trust,
    Time,
    Window,
}

pub struct State {
    pub now: i64,
    pub user: UserRecord,
    pub context: Option<CertificateLoginContext>,
    pub proofs: HashMap<String, StoredCertificateLogin>,
    pub mfa: HashMap<String, (MfaChallenge, i64)>,
    pub sessions: HashMap<String, (SessionPolicy, SessionState)>,
    pub events: Vec<ChainedEvent>,
    pub next_token: u64,
    pub verifier_calls: usize,
    pub factor_calls: usize,
    pub secret_exposures: usize,
    pub mfa_created: usize,
    pub sessions_created: usize,
    pub revocations: usize,
    pub touches: usize,
    pub proof_takes: usize,
    pub start_admissions: usize,
    pub proof_admissions: usize,
    pub authority_reads: usize,
    pub fail_start: bool,
    pub fail_proof: bool,
    pub fail_verify: bool,
    pub fail_audit: bool,
    pub fail_authority: bool,
    pub check_fault: Option<CheckFault>,
    pub after_verify: Option<Change>,
    pub after_factor: Option<Change>,
    pub after_mfa_create: Option<Change>,
    pub after_session_create: Option<Change>,
    pub after_touch: Option<Change>,
    pub after_user_find: Option<Change>,
}

impl State {
    fn new() -> Self {
        Self {
            now: 1000,
            user: user(),
            context: Some(context()),
            proofs: HashMap::new(),
            mfa: HashMap::new(),
            sessions: HashMap::new(),
            events: Vec::new(),
            next_token: 0,
            verifier_calls: 0,
            factor_calls: 0,
            secret_exposures: 0,
            mfa_created: 0,
            sessions_created: 0,
            revocations: 0,
            touches: 0,
            proof_takes: 0,
            start_admissions: 0,
            proof_admissions: 0,
            authority_reads: 0,
            fail_start: false,
            fail_proof: false,
            fail_verify: false,
            fail_audit: false,
            fail_authority: false,
            check_fault: None,
            after_verify: None,
            after_factor: None,
            after_mfa_create: None,
            after_session_create: None,
            after_touch: None,
            after_user_find: None,
        }
    }

    pub fn change(&mut self, change: Change) {
        match change {
            Change::Withdraw => self.context = None,
            Change::Advance(value) => self.now = value,
            Change::Principal => self.user.email = "changed@example.test".into(),
            Change::Generation => self.user.auth_generation += 1,
            Change::Trust => {
                let trust = &mut self.context.as_mut().unwrap().trust;
                trust.revision = trust.revision.next().unwrap();
                trust.inspection.crl_der = b"successor-crl".to_vec();
                trust.inspection.crl_digest = Hasher.hash_bytes(&trust.inspection.crl_der);
                trust.inspection.crl_number += 1;
            }
        }
        if let Some(context) = &mut self.context {
            context.account.principal = Principal::from(&self.user);
            context.account.auth_generation = self.user.auth_generation;
            context.account.revision = self.user.revision;
        }
    }

    pub fn token(&mut self, kind: &str) -> String {
        self.next_token += 1;
        format!("{kind}-{}", self.next_token)
    }
}

#[derive(Clone)]
pub struct Env(pub Arc<Mutex<State>>);

impl Env {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(State::new())))
    }

    pub fn edit(&self, f: impl FnOnce(&mut State)) {
        f(&mut self.0.lock().unwrap());
    }

    pub fn read<T>(&self, f: impl FnOnce(&State) -> T) -> T {
        f(&self.0.lock().unwrap())
    }

    fn identity_ports(&self) -> IdentityPorts {
        let shared = Arc::new(self.clone());
        IdentityPorts {
            users: shared.clone(),
            sessions: shared.clone(),
            passwords: shared.clone(),
            totp: shared.clone(),
            recovery: shared.clone(),
            secrets: shared.clone(),
            clock: shared,
            audit_log: Box::new(self.clone()),
        }
    }

    pub fn service(&self, guarded: bool, policy: SessionPolicy) -> IdentityService {
        if guarded {
            let shared = Arc::new(self.clone());
            IdentityService::with_certificate_login(
                self.identity_ports(),
                policy,
                CertificateLoginPorts {
                    authority: shared.clone(),
                    runtime: shared.clone(),
                    verifier: shared,
                    hasher: Arc::new(Hasher),
                },
            )
        } else {
            IdentityService::with_session_policy(self.identity_ports(), policy)
        }
    }

    pub fn begin(&self, service: &IdentityService) -> CertificateLoginChallenge {
        service
            .start_certificate_login(user().id, binding())
            .unwrap()
    }

    pub fn proof(&self, service: &IdentityService) -> LoginChallenge {
        let challenge = self.begin(service);
        service
            .prove_certificate_login(&challenge.challenge_token, &[5; 384])
            .unwrap()
    }

    pub fn certificate_session(&self, service: &IdentityService) -> SessionResult {
        let challenge = self.proof(service);
        service
            .complete_totp(&challenge.challenge_token, "123456")
            .unwrap()
    }

    pub fn password_session(&self, service: &IdentityService) -> SessionResult {
        let challenge = service
            .start_login("owner@example.test", "password")
            .unwrap();
        service
            .complete_totp(&challenge.challenge_token, "123456")
            .unwrap()
    }
}
