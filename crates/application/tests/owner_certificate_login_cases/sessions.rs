use domain::identity::UserId;

use super::{
    ApplicationError, CertificateMfaChallenge, CertificateSessionProvenance, Env,
    LoginChallengeIdentity, MfaChallenge, SessionAuthentication, SessionGrant, SessionIdentity,
    SessionPolicy, SessionState, SessionStore, State,
};

fn live(state: &mut State, token: &str, policy: SessionPolicy) -> Option<SessionState> {
    let now = state.now * 1000;
    let (configured, saved) = state.sessions.get_mut(token)?;
    if *configured != policy
        || now >= saved.absolute_expires_at_unix_ms
        || saved
            .idle_expires_at_unix_ms
            .is_some_and(|expires| now >= expires)
    {
        state.sessions.remove(token);
        return None;
    }
    saved.server_now_unix_ms = now;
    Some(saved.clone())
}

fn issue(
    state: &mut State,
    identity: &SessionIdentity,
    authentication: SessionAuthentication,
    policy: SessionPolicy,
    ceiling: i64,
) -> SessionGrant {
    let now = state.now * 1000;
    let absolute = (now + policy.absolute_ttl_seconds() as i64 * 1000).min(ceiling);
    assert!(absolute > now);
    let saved = SessionState {
        identity: identity.clone(),
        authentication,
        server_now_unix_ms: now,
        absolute_expires_at_unix_ms: absolute,
        idle_expires_at_unix_ms: policy
            .idle_ttl_seconds()
            .map(|ttl| (now + ttl as i64 * 1000).min(absolute)),
    };
    let token = state.token("session");
    state.sessions_created += 1;
    state
        .sessions
        .insert(token.clone(), (policy, saved.clone()));
    if let Some(change) = state.after_session_create.take() {
        state.change(change);
    }
    SessionGrant {
        access_token: token,
        state: saved,
    }
}

fn touch(
    state: &mut State,
    token: &str,
    expected: &SessionIdentity,
    authentication: SessionAuthentication,
    policy: SessionPolicy,
) -> Option<SessionState> {
    let mut saved = live(state, token, policy)?;
    if saved.identity != *expected || saved.authentication != authentication {
        return None;
    }
    state.touches += 1;
    saved.idle_expires_at_unix_ms = policy
        .idle_ttl_seconds()
        .map(|ttl| (state.now * 1000 + ttl as i64 * 1000).min(saved.absolute_expires_at_unix_ms));
    state
        .sessions
        .insert(token.to_owned(), (policy, saved.clone()));
    if let Some(change) = state.after_touch.take() {
        state.change(change);
    }
    Some(saved)
}

impl SessionStore for Env {
    fn create_challenge(
        &self,
        identity: &LoginChallengeIdentity,
        ttl: u64,
    ) -> Result<String, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        let token = state.token("password-mfa");
        let expires = state.now + ttl as i64;
        state.mfa.insert(
            token.clone(),
            (MfaChallenge::Password(identity.clone()), expires),
        );
        state.mfa_created += 1;
        Ok(token)
    }

    fn take_challenge(
        &self,
        token: &str,
    ) -> Result<Option<LoginChallengeIdentity>, ApplicationError> {
        Ok(match self.take_mfa_challenge(token)? {
            Some(MfaChallenge::Password(identity)) => Some(identity),
            _ => None,
        })
    }

    fn create_certificate_challenge(
        &self,
        value: &CertificateMfaChallenge,
    ) -> Result<String, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        assert!(value.expires_at_unix_seconds > state.now);
        assert!(value.expires_at_unix_seconds <= value.provenance.valid_until_unix_seconds);
        let token = state.token("certificate-mfa");
        state.mfa.insert(
            token.clone(),
            (
                MfaChallenge::Certificate(value.clone().into()),
                value.expires_at_unix_seconds,
            ),
        );
        state.mfa_created += 1;
        if let Some(change) = state.after_mfa_create.take() {
            state.change(change);
        }
        Ok(token)
    }

    fn take_mfa_challenge(&self, token: &str) -> Result<Option<MfaChallenge>, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        Ok(state
            .mfa
            .remove(token)
            .filter(|(_, expires)| state.now < *expires)
            .map(|v| v.0))
    }

    fn create_session(
        &self,
        identity: &SessionIdentity,
        policy: SessionPolicy,
    ) -> Result<SessionGrant, ApplicationError> {
        Ok(issue(
            &mut self.0.lock().unwrap(),
            identity,
            SessionAuthentication::Password,
            policy,
            i64::MAX,
        ))
    }

    fn create_certificate_session(
        &self,
        identity: &SessionIdentity,
        provenance: &CertificateSessionProvenance,
        policy: SessionPolicy,
        ceiling_unix_ms: i64,
    ) -> Result<SessionGrant, ApplicationError> {
        assert_eq!(identity.principal, provenance.principal);
        assert_eq!(identity.auth_generation, provenance.auth_generation);
        assert!(ceiling_unix_ms <= provenance.valid_until_unix_seconds * 1000);
        Ok(issue(
            &mut self.0.lock().unwrap(),
            identity,
            SessionAuthentication::Certificate(provenance.clone().into()),
            policy,
            ceiling_unix_ms,
        ))
    }

    fn find_session(
        &self,
        token: &str,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError> {
        Ok(live(&mut self.0.lock().unwrap(), token, policy))
    }

    fn record_activity(
        &self,
        token: &str,
        expected: &SessionIdentity,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError> {
        Ok(touch(
            &mut self.0.lock().unwrap(),
            token,
            expected,
            SessionAuthentication::Password,
            policy,
        ))
    }

    fn record_certificate_activity(
        &self,
        token: &str,
        expected: &SessionIdentity,
        provenance: &CertificateSessionProvenance,
        policy: SessionPolicy,
    ) -> Result<Option<SessionState>, ApplicationError> {
        Ok(touch(
            &mut self.0.lock().unwrap(),
            token,
            expected,
            SessionAuthentication::Certificate(provenance.clone().into()),
            policy,
        ))
    }

    fn revoke_session(&self, token: &str) -> Result<(), ApplicationError> {
        self.edit(|s| {
            s.revocations += 1;
            s.sessions.remove(token);
        });
        Ok(())
    }

    fn failed_password_attempts(&self, _: &str, _: u64) -> Result<u32, ApplicationError> {
        Ok(0)
    }
    fn record_password_failure(&self, _: &str, _: u64) -> Result<u32, ApplicationError> {
        Ok(1)
    }
    fn clear_password_failures(&self, _: &str) -> Result<(), ApplicationError> {
        Ok(())
    }
    fn claim_totp(&self, _: UserId, _: &str, _: u64) -> Result<bool, ApplicationError> {
        Ok(true)
    }
}
