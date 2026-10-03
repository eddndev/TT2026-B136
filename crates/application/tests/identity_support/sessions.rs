use application::identity::{
    certificate_login::{
        CertificateMfaChallenge, CertificateSessionProvenance, MfaChallenge, SessionAuthentication,
    },
    LoginChallengeIdentity, SessionGrant, SessionIdentity, SessionPolicy, SessionState,
    SessionStore,
};
use application::ApplicationError;
use domain::identity::UserId;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering},
    Mutex,
};

#[derive(Default)]
pub struct MemorySessions {
    pub(super) issued_sessions: AtomicUsize,
    pub(super) fail_challenge_cleanup: AtomicBool,
    pub(super) fail_session_cleanup: AtomicBool,
    pub(super) challenges: Mutex<HashMap<String, LoginChallengeIdentity>>,
    pub(super) sessions: Mutex<HashMap<String, (SessionPolicy, SessionState)>>,
    now: AtomicI64,
    pub(super) activity_calls: AtomicUsize,
    pub(super) fail_activity: AtomicBool,
    failures: Mutex<HashMap<String, u32>>,
    used_totp: Mutex<Vec<(UserId, String)>>,
}

impl SessionStore for MemorySessions {
    fn create_challenge(
        &self,
        identity: &LoginChallengeIdentity,
        _ttl: u64,
    ) -> Result<String, ApplicationError> {
        let token = format!("challenge-{}", uuid::Uuid::new_v4());
        self.challenges
            .lock()
            .unwrap()
            .insert(token.clone(), identity.clone());
        Ok(token)
    }

    fn take_challenge(
        &self,
        token: &str,
    ) -> Result<Option<LoginChallengeIdentity>, ApplicationError> {
        if self.fail_challenge_cleanup.load(Ordering::SeqCst) {
            return Err(ApplicationError::Port(format!(
                "cleanup failed for {token}"
            )));
        }
        Ok(self.challenges.lock().unwrap().remove(token))
    }

    fn create_certificate_challenge(
        &self,
        _: &CertificateMfaChallenge,
    ) -> Result<String, ApplicationError> {
        Err(ApplicationError::InvalidCredentials)
    }

    fn take_mfa_challenge(&self, token: &str) -> Result<Option<MfaChallenge>, ApplicationError> {
        Ok(self.take_challenge(token)?.map(MfaChallenge::Password))
    }

    fn create_certificate_session(
        &self,
        _: &SessionIdentity,
        _: &CertificateSessionProvenance,
        _: SessionPolicy,
        _: i64,
    ) -> Result<SessionGrant, ApplicationError> {
        Err(ApplicationError::InvalidSession)
    }

    fn record_certificate_activity(
        &self,
        _: &str,
        _: &SessionIdentity,
        _: &CertificateSessionProvenance,
        _: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError> {
        Err(ApplicationError::InvalidSession)
    }

    fn create_session(
        &self,
        identity: &SessionIdentity,
        policy: SessionPolicy,
    ) -> Result<SessionGrant, ApplicationError> {
        let index = self.issued_sessions.fetch_add(1, Ordering::SeqCst);
        let token = format!("session-{}-{index}", identity.principal.id);
        let now = self.now.load(Ordering::SeqCst);
        let state = SessionState {
            identity: identity.clone(),
            authentication: SessionAuthentication::Password,
            server_now_unix_ms: now,
            absolute_expires_at_unix_ms: now + policy.absolute_ttl_seconds() as i64 * 1000,
            idle_expires_at_unix_ms: policy
                .idle_ttl_seconds()
                .map(|idle| now + idle as i64 * 1000),
        };
        self.sessions
            .lock()
            .unwrap()
            .insert(token.clone(), (policy, state.clone()));
        Ok(SessionGrant {
            access_token: token,
            state,
        })
    }

    fn find_session(
        &self,
        token: &str,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError> {
        let mut sessions = self.sessions.lock().unwrap();
        Ok(lookup(
            &mut sessions,
            token,
            policy,
            self.now.load(Ordering::SeqCst),
        ))
    }

    fn record_activity(
        &self,
        token: &str,
        expected: &SessionIdentity,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError> {
        self.activity_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_activity.load(Ordering::SeqCst) {
            return Err(ApplicationError::Port("activity store unavailable".into()));
        }
        let mut sessions = self.sessions.lock().unwrap();
        let now = self.now.load(Ordering::SeqCst);
        let Some(mut state) = lookup(&mut sessions, token, policy, now) else {
            return Ok(None);
        };
        if &state.identity != expected || state.authentication != SessionAuthentication::Password {
            return Ok(None);
        }
        state.idle_expires_at_unix_ms = policy
            .idle_ttl_seconds()
            .map(|idle| (now + idle as i64 * 1000).min(state.absolute_expires_at_unix_ms));
        sessions.insert(token.into(), (policy, state.clone()));
        Ok(Some(state))
    }

    fn revoke_session(&self, token: &str) -> Result<(), ApplicationError> {
        if self.fail_session_cleanup.load(Ordering::SeqCst) {
            return Err(ApplicationError::Port(format!(
                "cleanup failed for {token}"
            )));
        }
        self.sessions.lock().unwrap().remove(token);
        Ok(())
    }

    fn failed_password_attempts(&self, email: &str, _ttl: u64) -> Result<u32, ApplicationError> {
        Ok(*self.failures.lock().unwrap().get(email).unwrap_or(&0))
    }

    fn record_password_failure(&self, email: &str, _ttl: u64) -> Result<u32, ApplicationError> {
        let mut failures = self.failures.lock().unwrap();
        let count = failures.entry(email.to_string()).or_default();
        *count += 1;
        Ok(*count)
    }

    fn clear_password_failures(&self, email: &str) -> Result<(), ApplicationError> {
        self.failures.lock().unwrap().remove(email);
        Ok(())
    }

    fn claim_totp(
        &self,
        user_id: UserId,
        code_fingerprint: &str,
        _ttl: u64,
    ) -> Result<bool, ApplicationError> {
        let key = (user_id, code_fingerprint.to_string());
        let mut used = self.used_totp.lock().unwrap();
        if used.contains(&key) {
            return Ok(false);
        }
        used.push(key);
        Ok(true)
    }
}

impl MemorySessions {
    pub(super) fn set_now(&self, now: i64) {
        self.now.store(now, Ordering::SeqCst);
    }
}

fn lookup(
    sessions: &mut HashMap<String, (SessionPolicy, SessionState)>,
    token: &str,
    policy: SessionPolicy,
    now: i64,
) -> Option<SessionState> {
    let (stored_policy, mut state) = sessions.get(token)?.clone();
    if stored_policy != policy
        || now >= state.absolute_expires_at_unix_ms
        || state
            .idle_expires_at_unix_ms
            .is_some_and(|deadline| now >= deadline)
    {
        sessions.remove(token);
        return None;
    }
    state.server_now_unix_ms = now;
    Some(state)
}
