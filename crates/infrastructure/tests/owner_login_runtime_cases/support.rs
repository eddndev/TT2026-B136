use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

pub use application::identity::certificate_login::{OwnerLoginRuntime, StoredCertificateLogin};
pub use application::ApplicationError;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use domain::crypto::DocumentHasher;
pub use domain::identity::UserId;
pub use infrastructure::identity::{
    OwnerLoginRateLimit, OwnerLoginRatePolicy, RedisOwnerLoginRuntime,
};
use infrastructure::RingSha256Hasher;
use redis::Commands;
pub use uuid::Uuid;

pub const START_GLOBAL: &str = "identity:certificate-login-rate:v1:start:global";
pub const PROOF_GLOBAL: &str = "identity:certificate-login-rate:v1:proof:global";
static SERIAL: Mutex<()> = Mutex::new(());

pub fn limit(maximum: u32, seconds: u64) -> OwnerLoginRateLimit {
    OwnerLoginRateLimit::new(maximum, seconds).unwrap()
}

pub fn policy(global: u32, subject: u32) -> OwnerLoginRatePolicy {
    OwnerLoginRatePolicy::new(
        limit(global, 60),
        limit(subject, 30),
        limit(global, 90),
        limit(subject, 45),
    )
}

pub fn connect(url: &str, policy: OwnerLoginRatePolicy) -> RedisOwnerLoginRuntime {
    RedisOwnerLoginRuntime::connect(url, policy, Duration::from_secs(2), Duration::from_secs(2))
        .unwrap()
}

pub fn capture_key(token: &str) -> String {
    format!(
        "identity:certificate-login:{}",
        RingSha256Hasher.hash_bytes(token.as_bytes()).to_hex()
    )
}

pub fn start_key(owner: UserId, binding: Uuid) -> String {
    let mut bytes = b"qadra:owner-certificate-login-start-limit:v1\0".to_vec();
    bytes.extend_from_slice(owner.as_uuid().as_bytes());
    bytes.extend_from_slice(binding.as_bytes());
    format!(
        "identity:certificate-login-rate:v1:start:owner-binding:{}",
        RingSha256Hasher.hash_bytes(&bytes).to_hex()
    )
}

pub fn proof_key(token: &str) -> String {
    let mut bytes = b"qadra:owner-certificate-login-proof-limit:v1\0".to_vec();
    bytes.extend_from_slice(token.as_bytes());
    format!(
        "identity:certificate-login-rate:v1:proof:token:{}",
        RingSha256Hasher.hash_bytes(&bytes).to_hex()
    )
}

pub fn token_shape(token: &str) {
    assert_eq!(token.len(), 43, "opaque proof token must have 32 bytes");
    let bytes = URL_SAFE_NO_PAD
        .decode(token)
        .expect("opaque proof token must be base64url");
    assert_eq!(bytes.len(), 32);
    assert!(
        URL_SAFE_NO_PAD.encode(bytes) == token,
        "noncanonical opaque proof token"
    );
}

#[derive(Debug, PartialEq, Eq)]
pub struct Snapshot {
    bytes: Option<Vec<u8>>,
    deadline: i64,
}

pub struct Fixture {
    pub url: String,
    pub connection: redis::Connection,
    owned: BTreeSet<String>,
    _serial: MutexGuard<'static, ()>,
}

impl Fixture {
    pub fn new() -> Self {
        let serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let url = std::env::var("IDENTITY_TEST_REDIS_URL")
            .expect("owner login runtime requires disposable IDENTITY_TEST_REDIS_URL");
        let parsed = redis::parse_redis_url(&url).expect("invalid disposable Redis URL");
        assert!(
            matches!(
                parsed.host_str(),
                Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
            ),
            "owner login runtime fixture requires loopback Redis"
        );
        let mut connection = redis::Client::open(url.as_str())
            .unwrap()
            .get_connection_with_timeout(Duration::from_secs(2))
            .unwrap();
        connection
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        connection
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let existing: u64 = redis::cmd("EXISTS")
            .arg(START_GLOBAL)
            .arg(PROOF_GLOBAL)
            .query(&mut connection)
            .unwrap();
        assert_eq!(
            existing, 0,
            "fixture will not claim existing login global budgets"
        );
        Self {
            url,
            connection,
            owned: [START_GLOBAL.into(), PROOF_GLOBAL.into()].into(),
            _serial: serial,
        }
    }

    pub fn store(&self, policy: OwnerLoginRatePolicy) -> RedisOwnerLoginRuntime {
        connect(&self.url, policy)
    }

    pub fn reserve(&mut self, key: String) -> String {
        assert!(
            !self.connection.exists::<_, bool>(&key).unwrap(),
            "fixture key collision"
        );
        self.owned.insert(key.clone());
        key
    }

    pub fn subject(&mut self) -> (UserId, Uuid, String) {
        let owner = UserId::new();
        let binding = Uuid::new_v4();
        let key = self.reserve(start_key(owner, binding));
        (owner, binding, key)
    }

    pub fn token(&mut self) -> (String, String) {
        let mut bytes = Uuid::new_v4().as_bytes().to_vec();
        bytes.extend_from_slice(Uuid::new_v4().as_bytes());
        let token = URL_SAFE_NO_PAD.encode(bytes);
        let key = self.reserve(capture_key(&token));
        self.reserve(proof_key(&token));
        (token, key)
    }

    pub fn issue(&mut self, value: &StoredCertificateLogin, ttl: u64) -> (String, String) {
        let token = self.store(policy(20, 10)).create(value, ttl).unwrap();
        token_shape(&token);
        let key = capture_key(&token);
        self.owned.insert(key.clone());
        (token, key)
    }

    pub fn now(&mut self) -> i64 {
        let (seconds, micros): (i64, i64) = redis::cmd("TIME").query(&mut self.connection).unwrap();
        seconds * 1000 + micros / 1000
    }

    pub fn deadline(&mut self, key: &str) -> i64 {
        redis::cmd("PEXPIRETIME")
            .arg(key)
            .query(&mut self.connection)
            .unwrap()
    }

    pub fn expire(&mut self, key: &str, deadline: i64) {
        redis::cmd("PEXPIREAT")
            .arg(key)
            .arg(deadline)
            .query::<()>(&mut self.connection)
            .unwrap();
    }

    pub fn string(&mut self, key: &str, value: &str, deadline: i64) {
        self.connection.set::<_, _, ()>(key, value).unwrap();
        self.expire(key, deadline);
    }

    pub fn read(&mut self, key: &str) -> Snapshot {
        Snapshot {
            bytes: redis::cmd("DUMP")
                .arg(key)
                .query(&mut self.connection)
                .unwrap(),
            deadline: self.deadline(key),
        }
    }

    pub fn snapshot(&mut self) -> BTreeMap<String, Snapshot> {
        self.owned
            .clone()
            .into_iter()
            .map(|key| {
                let value = self.read(&key);
                (key, value)
            })
            .collect()
    }

    pub fn fields(&mut self, key: &str) -> BTreeMap<String, String> {
        self.connection.hgetall(key).unwrap()
    }

    pub fn keys(&mut self) -> BTreeSet<String> {
        let mut cursor = 0_u64;
        let mut keys = BTreeSet::new();
        loop {
            let (next, page): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg("identity:*")
                .arg("COUNT")
                .arg(100)
                .query(&mut self.connection)
                .unwrap();
            keys.extend(page);
            if next == 0 {
                return keys;
            }
            cursor = next;
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for key in &self.owned {
            let _ = self.connection.del::<_, ()>(key);
        }
    }
}

pub fn exhausted(result: Result<(), ApplicationError>) {
    assert!(matches!(result, Err(ApplicationError::AccountLocked)));
}
