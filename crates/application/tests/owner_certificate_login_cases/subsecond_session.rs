use std::sync::{
    atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering},
    Arc,
};

use domain::{
    clock::{Clock, OffsetDateTime},
    identity::UserId,
};
use uuid::Uuid;

use crate::support::*;

const READ_TIME: i64 = 1_001_100;
const DEADLINE: i64 = 1_001_200;
const AFTER_WAIT: i64 = 1_001_800;

mockall::mock! {
    ReadSessions {}
    impl SessionStore for ReadSessions {
        fn create_challenge(&self, identity: &LoginChallengeIdentity, ttl: u64)
            -> Result<String, ApplicationError>;
        fn take_challenge(&self, token: &str)
            -> Result<Option<LoginChallengeIdentity>, ApplicationError>;
        fn create_certificate_challenge(&self, value: &CertificateMfaChallenge)
            -> Result<String, ApplicationError>;
        fn take_mfa_challenge(&self, token: &str)
            -> Result<Option<MfaChallenge>, ApplicationError>;
        fn create_session(&self, identity: &SessionIdentity, policy: SessionPolicy)
            -> Result<SessionGrant, ApplicationError>;
        fn create_certificate_session(&self, identity: &SessionIdentity,
            provenance: &CertificateSessionProvenance, policy: SessionPolicy,
            ceiling_unix_ms: i64) -> Result<SessionGrant, ApplicationError>;
        fn find_session(&self, token: &str, policy: SessionPolicy)
            -> Result<Option<SessionState>, ApplicationError>;
        fn record_certificate_activity(&self, token: &str, expected: &SessionIdentity,
            provenance: &CertificateSessionProvenance, policy: SessionPolicy)
            -> Result<Option<SessionState>, ApplicationError>;
        fn record_activity(&self, token: &str, expected: &SessionIdentity,
            policy: SessionPolicy) -> Result<Option<SessionState>, ApplicationError>;
        fn revoke_session(&self, token: &str) -> Result<(), ApplicationError>;
        fn failed_password_attempts(&self, email: &str, ttl: u64)
            -> Result<u32, ApplicationError>;
        fn record_password_failure(&self, email: &str, ttl: u64)
            -> Result<u32, ApplicationError>;
        fn clear_password_failures(&self, email: &str) -> Result<(), ApplicationError>;
        fn claim_totp(&self, user_id: UserId, code: &str, ttl: u64)
            -> Result<bool, ApplicationError>;
    }
}

struct FractionalClock(AtomicI64);

impl Clock for FractionalClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp_nanos(
            i128::from(self.0.load(Ordering::SeqCst)) * 1_000_000,
        )
        .unwrap()
    }
}

struct WaitingAuthority {
    env: Env,
    clock: Arc<FractionalClock>,
    session_reads: Arc<AtomicUsize>,
    waited: AtomicBool,
}

impl OwnerLoginAuthority for WaitingAuthority {
    fn load(
        &self,
        owner: UserId,
        binding: Uuid,
    ) -> Result<Option<CertificateLoginContext>, ApplicationError> {
        if self.session_reads.load(Ordering::SeqCst) >= 2 {
            self.clock.0.store(AFTER_WAIT, Ordering::SeqCst);
            self.waited.store(true, Ordering::SeqCst);
        }
        self.env.load(owner, binding)
    }
}

fn rejects_deadline_crossed_during_authority_wait(idle: bool) {
    let env = Env::new();
    let policy = SessionPolicy::new(if idle { 60 } else { 1 }, idle.then_some(1)).unwrap();
    let issued = env.certificate_session(&env.service(true, policy));
    let mut stored = env.read(|s| s.sessions[&issued.access_token].1.clone());
    if idle {
        stored.idle_expires_at_unix_ms = Some(DEADLINE);
    } else {
        stored.absolute_expires_at_unix_ms = DEADLINE;
    }
    let clock = Arc::new(FractionalClock(AtomicI64::new(READ_TIME)));
    let reads = Arc::new(AtomicUsize::new(0));
    let authority = Arc::new(WaitingAuthority {
        env: env.clone(),
        clock: clock.clone(),
        session_reads: reads.clone(),
        waited: AtomicBool::new(false),
    });
    let mut sessions = MockReadSessions::new();
    let expected_token = issued.access_token.clone();
    let read_clock = clock.clone();
    let read_count = reads.clone();
    sessions
        .expect_find_session()
        .withf(move |token, configured| token == expected_token && *configured == policy)
        .times(2..)
        .returning(move |_, _| {
            read_count.fetch_add(1, Ordering::SeqCst);
            let now = read_clock.0.load(Ordering::SeqCst);
            if now >= stored.absolute_expires_at_unix_ms
                || stored.idle_expires_at_unix_ms.is_some_and(|end| now >= end)
            {
                return Ok(None);
            }
            let mut current = stored.clone();
            current.server_now_unix_ms = now;
            Ok(Some(current))
        });
    let shared = Arc::new(env.clone());
    let service = IdentityService::with_certificate_login(
        IdentityPorts {
            users: shared.clone(),
            sessions: Arc::new(sessions),
            passwords: shared.clone(),
            totp: shared.clone(),
            recovery: shared.clone(),
            secrets: shared.clone(),
            clock: clock.clone(),
            audit_log: Box::new(env.clone()),
        },
        policy,
        CertificateLoginPorts {
            authority: authority.clone(),
            runtime: shared.clone(),
            verifier: shared,
            hasher: Arc::new(Hasher),
        },
    );
    let before = env.read(|s| (s.context.clone(), s.events.len(), s.touches, s.revocations));
    let result = service.authenticate(&issued.access_token);
    assert!(authority.waited.load(Ordering::SeqCst));
    assert_eq!(clock.0.load(Ordering::SeqCst), AFTER_WAIT);
    assert_eq!(
        env.read(|s| (s.context.clone(), s.events.len(), s.touches, s.revocations)),
        before,
    );
    assert!(
        matches!(result, Err(ApplicationError::InvalidSession)),
        "session admission must reject a deadline crossed within the final authority wait"
    );
}

#[test]
fn idle_deadline_crossed_within_the_final_authority_wait_rejects_session() {
    rejects_deadline_crossed_during_authority_wait(true);
}

#[test]
fn absolute_deadline_crossed_within_the_final_authority_wait_rejects_session() {
    rejects_deadline_crossed_during_authority_wait(false);
}
