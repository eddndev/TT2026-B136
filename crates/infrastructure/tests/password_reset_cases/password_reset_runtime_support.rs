pub use application::identity::password_reset::{PasswordResetLimiter, ResetTokenSource};
use domain::crypto::{DocumentHasher, Sha256Digest};
pub use infrastructure::identity::{
    PasswordResetRateLimit, PasswordResetRatePolicy, RandomResetTokenSource,
    RedisPasswordResetLimiter,
};
use infrastructure::RingSha256Hasher;
use redis::Commands;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

pub const REQUEST_GLOBAL: &str = "identity:password-reset:v1:request:global";
pub const COMPLETION_GLOBAL: &str = "identity:password-reset:v1:completion:global";
static FIXTURE_LOCK: Mutex<()> = Mutex::new(());

pub fn limit(max: u32, seconds: u64) -> PasswordResetRateLimit {
    PasswordResetRateLimit::new(max, seconds).unwrap()
}

pub fn policy(global: u32, subject: u32) -> PasswordResetRatePolicy {
    PasswordResetRatePolicy::new(
        limit(global, 60),
        limit(subject, 30),
        limit(global, 90),
        limit(subject, 45),
    )
}

fn digest(value: u8) -> Sha256Digest {
    let mut bytes = uuid::Uuid::new_v4().as_bytes().to_vec();
    bytes.push(value);
    RingSha256Hasher.hash_bytes(&bytes)
}

pub fn email() -> String {
    format!(
        "reset-budget-{}@example.test",
        uuid::Uuid::new_v4().simple()
    )
}

pub fn email_key(email: &str) -> String {
    key(
        "request:email",
        b"qadra:password-reset-request-limit:v1\0",
        email.as_bytes(),
    )
}

pub fn completion_key(digest: Sha256Digest) -> String {
    key(
        "completion:digest",
        b"qadra:password-reset-completion-limit:v1\0",
        digest.as_bytes(),
    )
}

fn key(kind: &str, context: &[u8], value: &[u8]) -> String {
    let mut bytes = Vec::from(context);
    bytes.extend_from_slice(value);
    format!(
        "identity:password-reset:v1:{kind}:{}",
        RingSha256Hasher.hash_bytes(&bytes).to_hex()
    )
}

#[derive(PartialEq, Eq)]
pub struct Stored {
    bytes: Option<Vec<u8>>,
    deadline: i64,
}

pub struct Fixture {
    pub url: String,
    pub connection: redis::Connection,
    owned: BTreeSet<String>,
    _guard: MutexGuard<'static, ()>,
}

impl Fixture {
    pub fn new() -> Self {
        let guard = FIXTURE_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let url = std::env::var("IDENTITY_TEST_REDIS_URL")
            .expect("reset runtime tests require disposable IDENTITY_TEST_REDIS_URL");
        let mut connection = redis::Client::open(url.as_str())
            .unwrap()
            .get_connection()
            .unwrap();
        let existing: u64 = redis::cmd("EXISTS")
            .arg(REQUEST_GLOBAL)
            .arg(COMPLETION_GLOBAL)
            .query(&mut connection)
            .unwrap();
        assert_eq!(
            existing, 0,
            "reset globals already exist; fixture will not claim them"
        );
        Self {
            url,
            connection,
            owned: [REQUEST_GLOBAL.into(), COMPLETION_GLOBAL.into()].into(),
            _guard: guard,
        }
    }

    pub fn store(&self, policy: PasswordResetRatePolicy) -> RedisPasswordResetLimiter {
        RedisPasswordResetLimiter::connect(
            &self.url,
            policy,
            Duration::from_secs(1),
            Duration::from_secs(1),
        )
        .unwrap()
    }

    pub fn track(&mut self, key: impl Into<String>) -> String {
        let key = key.into();
        self.owned.insert(key.clone());
        key
    }

    pub fn email(&mut self) -> (String, String) {
        let value = email();
        let key = self.reserve(email_key(&value));
        (value, key)
    }

    pub fn completion(&mut self, value: u8) -> (Sha256Digest, String) {
        let digest = digest(value);
        let key = self.reserve(completion_key(digest));
        (digest, key)
    }

    fn reserve(&mut self, key: String) -> String {
        assert!(
            !self.connection.exists::<_, bool>(&key).unwrap(),
            "reset subject already exists; fixture will not claim it"
        );
        self.track(key)
    }

    pub fn snapshot(&mut self) -> BTreeMap<String, Stored> {
        self.owned
            .clone()
            .into_iter()
            .map(|key| {
                let state = self.read(&key);
                (key, state)
            })
            .collect()
    }

    pub fn read(&mut self, key: &str) -> Stored {
        Stored {
            bytes: redis::cmd("DUMP")
                .arg(key)
                .query(&mut self.connection)
                .unwrap(),
            deadline: self.deadline(key),
        }
    }

    pub fn fields(&mut self, key: &str) -> BTreeMap<String, String> {
        self.connection.hgetall(key).unwrap()
    }

    pub fn deadline(&mut self, key: &str) -> i64 {
        redis::cmd("PEXPIRETIME")
            .arg(key)
            .query(&mut self.connection)
            .unwrap()
    }

    pub fn now(&mut self) -> i64 {
        let (seconds, micros): (i64, i64) = redis::cmd("TIME").query(&mut self.connection).unwrap();
        seconds * 1000 + micros / 1000
    }

    pub fn expire_at(&mut self, key: &str, deadline: i64) {
        redis::cmd("PEXPIREAT")
            .arg(key)
            .arg(deadline)
            .query::<()>(&mut self.connection)
            .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for key in &self.owned {
            let _ = self.connection.del::<_, ()>(key);
        }
    }
}
