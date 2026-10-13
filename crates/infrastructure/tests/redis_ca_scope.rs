use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;
use redis::Commands;

use super::support::{
    Fixture, MfaChallenge, RedisSessionStore, SessionAuthentication, SessionStore,
};

struct ScopedStores {
    db: Fixture,
    ca: RedisSessionStore,
    owned: Vec<String>,
}

impl ScopedStores {
    fn new() -> Self {
        let mut db = Fixture::new();
        db.identity.principal.email = format!("ca-scope-{}@example.test", db.identity.principal.id);
        db.origin.principal = db.identity.principal.clone();
        let ca = RedisSessionStore::connect_ca(&db.url).unwrap();
        Self {
            db,
            ca,
            owned: Vec::new(),
        }
    }

    fn stores(&self, ca: bool) -> (&RedisSessionStore, &RedisSessionStore) {
        if ca {
            (&self.ca, &self.db.store)
        } else {
            (&self.db.store, &self.ca)
        }
    }

    fn track(&mut self, namespace: &str, value: &[u8]) -> [String; 2] {
        let digest = RingSha256Hasher.hash_bytes(value).to_hex();
        let keys = [
            format!("identity:{namespace}:{digest}"),
            format!("ca-identity:{namespace}:{digest}"),
        ];
        self.owned.extend(keys.iter().cloned());
        keys
    }

    fn assert_issued_key(&mut self, namespace: &str, token: &str, ca: bool) {
        let keys = self.track(namespace, token.as_bytes());
        assert!(self
            .db
            .connection
            .exists::<_, bool>(&keys[usize::from(ca)])
            .unwrap());
        assert!(!self
            .db
            .connection
            .exists::<_, bool>(&keys[usize::from(!ca)])
            .unwrap());
    }
}

impl Drop for ScopedStores {
    fn drop(&mut self) {
        for key in &self.owned {
            let _ = self.db.connection.del::<_, ()>(key);
        }
    }
}

#[test]
fn password_challenges_cannot_be_claimed_or_consumed_by_the_other_application() {
    let mut scoped = ScopedStores::new();
    let identity = scoped.db.challenge().identity;
    for ca in [false, true] {
        let token = scoped.stores(ca).0.create_challenge(&identity, 60).unwrap();
        scoped.assert_issued_key("challenge", &token, ca);
        let (issuer, other) = scoped.stores(ca);
        assert!(other.take_challenge(&token).unwrap().is_none());
        assert!(other.take_mfa_challenge(&token).unwrap().is_none());
        assert_eq!(
            issuer.take_mfa_challenge(&token).unwrap(),
            Some(MfaChallenge::Password(identity.clone()))
        );
        assert!(issuer.take_challenge(&token).unwrap().is_none());
    }
}

#[test]
fn password_sessions_reject_cross_application_reads_activity_and_revocation() {
    let mut scoped = ScopedStores::new();
    let policy = scoped.db.policy;
    let identity = scoped.db.identity.clone();
    for ca in [false, true] {
        let grant = scoped
            .stores(ca)
            .0
            .create_session(&identity, policy)
            .unwrap();
        let token = &grant.access_token;
        scoped.assert_issued_key("session", token, ca);
        let (issuer, other) = scoped.stores(ca);
        assert!(other.find_session(token, policy).unwrap().is_none());
        assert!(other
            .record_activity(token, &identity, policy)
            .unwrap()
            .is_none());
        other.revoke_session(token).unwrap();
        let state = issuer.find_session(token, policy).unwrap().unwrap();
        assert_eq!(state.identity, identity);
        assert_eq!(state.authentication, SessionAuthentication::Password);
        assert!(issuer
            .record_activity(token, &identity, policy)
            .unwrap()
            .is_some());
        let reopened = if ca {
            RedisSessionStore::connect_ca(&scoped.db.url).unwrap()
        } else {
            RedisSessionStore::connect(&scoped.db.url).unwrap()
        };
        assert_eq!(
            reopened
                .find_session(token, policy)
                .unwrap()
                .unwrap()
                .identity,
            identity
        );
        reopened.revoke_session(token).unwrap();
        assert!(issuer.find_session(token, policy).unwrap().is_none());
    }
}

#[test]
fn certificate_challenges_and_sessions_keep_the_same_application_boundary() {
    let mut scoped = ScopedStores::new();
    let challenge = scoped.db.challenge();
    let identity = scoped.db.identity.clone();
    let origin = scoped.db.origin.clone();
    let policy = scoped.db.policy;
    let ceiling = scoped.db.now() + 60_000;
    for ca in [false, true] {
        let token = scoped
            .stores(ca)
            .0
            .create_certificate_challenge(&challenge)
            .unwrap();
        scoped.assert_issued_key("challenge", &token, ca);
        let (issuer, other) = scoped.stores(ca);
        assert!(other.take_mfa_challenge(&token).unwrap().is_none());
        assert_eq!(
            issuer.take_mfa_challenge(&token).unwrap(),
            Some(MfaChallenge::Certificate(challenge.clone().into()))
        );
        let token = issuer
            .create_certificate_session(&identity, &origin, policy, ceiling)
            .unwrap()
            .access_token;
        scoped.assert_issued_key("session", &token, ca);
        let (issuer, other) = scoped.stores(ca);
        assert!(other.find_session(&token, policy).unwrap().is_none());
        assert!(other
            .record_certificate_activity(&token, &identity, &origin, policy)
            .unwrap()
            .is_none());
        other.revoke_session(&token).unwrap();
        let state = issuer.find_session(&token, policy).unwrap().unwrap();
        assert_eq!(state.identity, identity);
        assert_eq!(
            state.authentication,
            SessionAuthentication::Certificate(origin.clone().into())
        );
        assert!(issuer
            .record_certificate_activity(&token, &identity, &origin, policy)
            .unwrap()
            .is_some());
        issuer.revoke_session(&token).unwrap();
        assert!(issuer.find_session(&token, policy).unwrap().is_none());
    }
}

#[test]
fn password_failure_counts_and_resets_do_not_cross_credential_realms() {
    let mut scoped = ScopedStores::new();
    let email = scoped.db.identity.principal.email.clone();
    let keys = scoped.track("password-failures", email.as_bytes());
    assert_eq!(
        scoped.db.store.record_password_failure(&email, 60).unwrap(),
        1
    );
    assert_eq!(scoped.ca.failed_password_attempts(&email, 60).unwrap(), 0);
    assert_eq!(scoped.ca.record_password_failure(&email, 60).unwrap(), 1);
    assert_eq!(scoped.ca.record_password_failure(&email, 60).unwrap(), 2);
    assert_eq!(
        scoped
            .db
            .store
            .failed_password_attempts(&email, 60)
            .unwrap(),
        1
    );
    for key in keys {
        assert!(scoped.db.connection.exists::<_, bool>(key).unwrap());
    }
    scoped.db.store.clear_password_failures(&email).unwrap();
    assert_eq!(scoped.ca.failed_password_attempts(&email, 60).unwrap(), 2);
    assert_eq!(
        scoped.db.store.record_password_failure(&email, 60).unwrap(),
        1
    );
    scoped.ca.clear_password_failures(&email).unwrap();
    assert_eq!(scoped.ca.failed_password_attempts(&email, 60).unwrap(), 0);
    assert_eq!(
        scoped
            .db
            .store
            .failed_password_attempts(&email, 60)
            .unwrap(),
        1
    );
}

#[test]
fn totp_replay_claims_are_single_use_within_each_separate_credential_realm() {
    let mut scoped = ScopedStores::new();
    let user = scoped.db.identity.principal.id;
    for ca_first in [false, true] {
        let code = if ca_first { "456789" } else { "123456" };
        let mut claim = user.as_uuid().as_bytes().to_vec();
        claim.extend_from_slice(code.as_bytes());
        let keys = scoped.track("totp-used", &claim);
        let (first, other) = scoped.stores(ca_first);
        assert!(first.claim_totp(user, code, 60).unwrap());
        assert!(!first.claim_totp(user, code, 60).unwrap());
        assert!(other.claim_totp(user, code, 60).unwrap());
        assert!(!other.claim_totp(user, code, 60).unwrap());
        for key in keys {
            assert!(scoped.db.connection.exists::<_, bool>(key).unwrap());
        }
    }
}
