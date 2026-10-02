use std::sync::Arc;

use application::identity::password_reset::{
    PasswordResetPorts, PasswordResetService, ResetPolicy,
};
use application::identity::{
    IdentityPorts, IdentityService, Principal, SessionIdentity, SessionPolicy, SessionResult,
    SessionStore, UserRepository,
};
use domain::audit::{AuditLog, ChainVerification};
use domain::crypto::{DocumentHasher, RECOVERY_CODE_COUNT};
use domain::identity::UserId;
use infrastructure::identity::PostgresPasswordResetRepository;
use infrastructure::{
    AesGcmSecretProtector, Argon2idHasher, PostgresAuditLog, PostgresUserRepository,
    RandomRecoveryCodeGenerator, RedisSessionStore, RingSha256Hasher, SystemClock, TotpRsProvider,
};
use redis::Commands;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::case_administration_support::Fixture;
use crate::password_reset_identity_doubles::{AdmitReset, CapturedDelivery, FixedResetToken};

pub fn ok<T, E>(result: Result<T, E>, message: &str) -> T {
    result.unwrap_or_else(|_| panic!("{message}"))
}

pub struct Acceptance {
    pub identity: IdentityService,
    pub reset: PasswordResetService,
    pub sessions: Arc<RedisSessionStore>,
    pub secrets: Arc<AesGcmSecretProtector>,
    pub delivery: Arc<CapturedDelivery>,
    pub policy: SessionPolicy,
    users: Arc<PostgresUserRepository>,
    keys: RedisKeys,
    pub db: Fixture,
}

impl Acceptance {
    pub fn new() -> Self {
        let redis_url = std::env::var("IDENTITY_TEST_REDIS_URL")
            .expect("password reset identity acceptance requires IDENTITY_TEST_REDIS_URL");
        let mut db = Fixture::old()
            .expect("password reset identity acceptance requires CASE_TEST_DATABASE_URL");
        // This Owner only authorizes enrollment; none of its recovery slots is usable.
        let owner_codes = serde_json::json!({
            "slots": vec![Option::<String>::None; RECOVERY_CODE_COUNT],
        });
        assert_eq!(
            ok(
                db.admin.execute(
                    "UPDATE users SET recovery_codes=$1 WHERE id=$2",
                    &[&owner_codes, &db.owner.as_uuid()],
                ),
                "prepare valid fixture owner recovery slots"
            ),
            1
        );
        db.migrate();
        let users = Arc::new(ok(
            PostgresUserRepository::open(&db.runtime_url),
            "open users",
        ));
        let sessions = Arc::new(ok(RedisSessionStore::connect(&redis_url), "open sessions"));
        let passwords = Arc::new(Argon2idHasher::new());
        let secrets = Arc::new(ok(
            AesGcmSecretProtector::new(Zeroizing::new(vec![42; 32])),
            "create disposable TOTP protection",
        ));
        let policy = ok(SessionPolicy::new(300, None), "session fixture policy");
        let identity = IdentityService::with_session_policy(
            IdentityPorts {
                users: users.clone(),
                sessions: sessions.clone(),
                passwords: passwords.clone(),
                totp: Arc::new(TotpRsProvider::new()),
                recovery: Arc::new(RandomRecoveryCodeGenerator),
                secrets: secrets.clone(),
                clock: Arc::new(SystemClock::new()),
                audit_log: Box::new(ok(PostgresAuditLog::open(&db.runtime_url), "open audit")),
            },
            policy,
        );
        let delivery = Arc::new(CapturedDelivery::default());
        let reset = PasswordResetService::new(
            PasswordResetPorts {
                repository: Arc::new(ok(
                    PostgresPasswordResetRepository::open(&db.runtime_url),
                    "open reset repository",
                )),
                delivery: delivery.clone(),
                limiter: Arc::new(AdmitReset),
                tokens: Arc::new(FixedResetToken),
                digests: Arc::new(RingSha256Hasher),
                passwords,
            },
            ok(ResetPolicy::new(300, 2), "reset fixture policy"),
        );
        Self {
            identity,
            reset,
            sessions,
            secrets,
            delivery,
            policy,
            users,
            keys: RedisKeys::new(&redis_url),
            db,
        }
    }

    /// The existing fixture Owner seeds authorization, not a password login.
    pub fn owner_session(&mut self) -> Zeroizing<String> {
        let owner = ok(self.users.find_by_id(self.db.owner), "load fixture owner")
            .expect("fixture owner exists");
        let grant = ok(
            self.sessions.create_session(
                &SessionIdentity {
                    principal: Principal::from(&owner),
                    auth_generation: owner.auth_generation,
                },
                self.policy,
            ),
            "seed fixture owner session",
        );
        let token = Zeroizing::new(grant.access_token);
        self.keys.track("session", token.as_bytes());
        token
    }

    pub fn track_email(&mut self, email: &str) {
        self.keys.track("password-failures", email.as_bytes());
    }

    pub fn challenge(&mut self, email: &str, password: &str) -> Zeroizing<String> {
        let challenge = ok(self.identity.start_login(email, password), "password login");
        let token = Zeroizing::new(challenge.challenge_token);
        self.keys.track("challenge", token.as_bytes());
        token
    }

    pub fn session(&mut self, result: SessionResult) -> Zeroizing<String> {
        let token = Zeroizing::new(result.access_token);
        self.keys.track("session", token.as_bytes());
        token
    }

    pub fn track_totp(&mut self, id: UserId, code: &str) {
        let mut claim = Zeroizing::new(id.as_uuid().as_bytes().to_vec());
        claim.extend_from_slice(code.as_bytes());
        self.keys.track("totp-used", &claim);
    }

    pub fn assign_case(&mut self, id: UserId) {
        ok(
            self.db.admin.execute(
                "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)",
                &[&self.db.case.as_uuid(), &id.as_uuid()],
            ),
            "prepare membership",
        );
    }

    pub fn snapshot(&mut self, id: UserId) -> Snapshot {
        let user =
            ok(self.users.find_by_id(id), "read identity").expect("enrolled identity exists");
        let principal = Principal::from(&user);
        let recovery = Zeroizing::new(ok(
            serde_json::to_string(&user.recovery_codes),
            "capture recovery slots",
        ));
        let memberships = ok(
            self.db.admin.query(
                "SELECT case_id FROM case_memberships WHERE user_id=$1 ORDER BY case_id",
                &[&id.as_uuid()],
            ),
            "read membership",
        )
        .iter()
        .map(|row| row.get(0))
        .collect();
        Snapshot {
            principal,
            active: user.active,
            hash: Zeroizing::new(user.password_hash),
            remaining: user.recovery_codes.remaining(),
            recovery,
            protected_totp: user.protected_totp_secret,
            revision: user.revision,
            generation: user.auth_generation,
            memberships,
        }
    }

    pub fn assert_reset_digest(&mut self, token: &[u8]) {
        let mut input = Zeroizing::new(b"qadra:password-reset:v1\0".to_vec());
        input.extend_from_slice(token);
        let expected = RingSha256Hasher.hash_bytes(&input);
        let rows = ok(
            self.db
                .admin
                .query("SELECT digest FROM password_reset_capabilities", &[]),
            "read reset digest",
        );
        assert_eq!(rows.len(), 1);
        let stored: Vec<u8> = rows[0].get(0);
        assert!(
            stored.as_slice() == expected.as_bytes(),
            "reset digest differs"
        );
        assert!(
            stored.as_slice() != token,
            "raw reset token persisted as digest"
        );
    }

    pub fn assert_audit(&self, id: UserId, revision: u64, generation: u64) {
        let audit = ok(
            PostgresAuditLog::open(&self.db.runtime_url),
            "open audit reader",
        );
        let events = ok(audit.load_all(), "load audited events");
        assert!(matches!(
            domain::audit::verify_chain(&RingSha256Hasher, &events),
            Ok(ChainVerification::Valid { entries }) if entries == events.len()
        ));
        let resets: Vec<_> = events
            .iter()
            .filter(|item| item.event.action == "identity.password_reset")
            .collect();
        assert_eq!(resets.len(), 1);
        assert_eq!(resets[0].event.actor, "password-reset");
        assert_eq!(
            resets[0].event.resource,
            format!("user:{id}:revision:{revision}:generation:{generation}")
        );
    }
}

pub struct Snapshot {
    pub principal: Principal,
    pub active: bool,
    pub hash: Zeroizing<String>,
    pub recovery: Zeroizing<String>,
    pub remaining: usize,
    pub protected_totp: Vec<u8>,
    pub revision: u64,
    pub generation: u64,
    pub memberships: Vec<Uuid>,
}

impl Snapshot {
    pub fn same_preserved_identity(&self, other: &Self) -> bool {
        self.principal == other.principal
            && self.active == other.active
            && self.recovery.as_str() == other.recovery.as_str()
            && self.protected_totp == other.protected_totp
            && self.memberships == other.memberships
    }

    pub fn consumed(&self, index: usize) -> bool {
        let value: serde_json::Value = ok(
            serde_json::from_str(&self.recovery),
            "read captured recovery slots",
        );
        value["slots"]
            .as_array()
            .expect("recovery slots are an array")
            .get(index)
            .is_some_and(serde_json::Value::is_null)
    }
}

struct RedisKeys {
    connection: redis::Connection,
    keys: Vec<String>,
}

impl RedisKeys {
    fn new(url: &str) -> Self {
        let client = ok(redis::Client::open(url), "open cleanup client");
        Self {
            connection: ok(client.get_connection(), "connect cleanup"),
            keys: Vec::new(),
        }
    }

    fn track(&mut self, namespace: &str, value: &[u8]) {
        self.keys.push(format!(
            "identity:{namespace}:{}",
            RingSha256Hasher.hash_bytes(value).to_hex()
        ));
    }
}

impl Drop for RedisKeys {
    fn drop(&mut self) {
        if !self.keys.is_empty() {
            let _ = self.connection.del::<_, ()>(&self.keys);
        }
    }
}
