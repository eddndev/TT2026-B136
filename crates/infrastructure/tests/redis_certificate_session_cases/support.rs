use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

pub use application::identity::certificate_login::{
    CertificateMfaChallenge, CertificateSessionProvenance, MfaChallenge, SessionAuthentication,
};
pub use application::identity::{
    LoginChallengeIdentity, Principal, SessionIdentity, SessionPolicy, SessionStore,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use domain::crypto::{DocumentHasher, Sha256Digest};
pub use domain::identity::{Role, UserId};
pub use infrastructure::RedisSessionStore;
use infrastructure::RingSha256Hasher;
use redis::Commands;
pub use serde_json::{json, Value};

static SERIAL: Mutex<()> = Mutex::new(());
pub const MAX_EXACT: i64 = 9_007_199_254_740_991;

pub struct Fixture {
    pub url: String,
    pub connection: redis::Connection,
    pub store: RedisSessionStore,
    pub identity: SessionIdentity,
    pub origin: CertificateSessionProvenance,
    pub policy: SessionPolicy,
    owned: BTreeSet<String>,
    _serial: MutexGuard<'static, ()>,
}

#[derive(PartialEq, Eq)]
pub struct Snapshot {
    kind: String,
    fields: Option<BTreeMap<Vec<u8>, Vec<u8>>>,
    bytes: Option<Vec<u8>>,
    deadline: i64,
}

impl Fixture {
    pub fn new() -> Self {
        let serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
        let url = std::env::var("IDENTITY_TEST_REDIS_URL")
            .expect("certificate session tests require disposable IDENTITY_TEST_REDIS_URL");
        let parsed = redis::parse_redis_url(&url).expect("invalid disposable Redis URL");
        assert!(
            matches!(
                parsed.host_str(),
                Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
            ),
            "certificate session fixture requires a loopback backend"
        );
        let store = connect(&url);
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
        let now = time(&mut connection) / 1000;
        let identity = SessionIdentity {
            principal: Principal {
                id: UserId::new(),
                email: "owner-session@example.test".into(),
                role: Role::Owner,
            },
            auth_generation: 9_007_199_254_740_993,
        };
        let origin = CertificateSessionProvenance {
            principal: identity.principal.clone(),
            auth_generation: identity.auth_generation,
            binding_id: uuid::Uuid::new_v4(),
            deployment_id: uuid::Uuid::new_v4(),
            trust_revision: 7,
            root_fingerprint: digest(1),
            leaf_fingerprint: digest(2),
            crl_digest: digest(3),
            valid_from_unix_seconds: now - 60,
            valid_until_unix_seconds: now + 3600,
        };
        Self {
            url,
            connection,
            store,
            identity,
            origin,
            policy: SessionPolicy::new(120, Some(30)).unwrap(),
            owned: BTreeSet::new(),
            _serial: serial,
        }
    }

    pub fn now(&mut self) -> i64 {
        time(&mut self.connection)
    }

    pub fn challenge(&mut self) -> CertificateMfaChallenge {
        CertificateMfaChallenge {
            identity: LoginChallengeIdentity {
                user_id: self.identity.principal.id,
                auth_generation: self.identity.auth_generation,
            },
            provenance: self.origin.clone(),
            expires_at_unix_seconds: self.now() / 1000 + 90,
        }
    }

    pub fn track(&mut self, namespace: &str, token: &str) -> String {
        let key = key(namespace, token);
        self.owned.insert(key.clone());
        key
    }

    pub fn reserve(&mut self, namespace: &str) -> (String, String) {
        let token = uuid::Uuid::new_v4().to_string();
        let key = self.track(namespace, &token);
        assert!(
            !self.connection.exists::<_, bool>(&key).unwrap(),
            "fixture key collision"
        );
        (token, key)
    }

    pub fn issue(&mut self, ceiling: i64) -> (String, String, application::identity::SessionState) {
        let grant = self
            .store
            .create_certificate_session(&self.identity, &self.origin, self.policy, ceiling)
            .unwrap();
        token_shape(&grant.access_token);
        let key = self.track("session", &grant.access_token);
        (grant.access_token, key, grant.state)
    }

    pub fn fields(&mut self, key: &str) -> BTreeMap<String, String> {
        self.connection.hgetall(key).unwrap()
    }

    pub fn snapshot(&mut self, key: &str) -> Snapshot {
        let kind: String = redis::cmd("TYPE")
            .arg(key)
            .query(&mut self.connection)
            .unwrap();
        // Hash serialization order can change after a read during rehashing.
        // Compare every field byte and the absolute deadline instead.
        let (fields, bytes) = match kind.as_str() {
            "hash" => (Some(self.connection.hgetall(key).unwrap()), None),
            "string" => (None, Some(self.connection.get(key).unwrap())),
            "none" => (None, None),
            _ => panic!("unsupported fixture snapshot type"),
        };
        Snapshot {
            kind,
            fields,
            bytes,
            deadline: self.deadline(key),
        }
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

    pub fn set(&mut self, key: &str, field: &str, value: impl redis::ToRedisArgs) {
        self.connection
            .hset::<_, _, _, ()>(key, field, value)
            .unwrap();
    }

    pub fn string(&mut self, key: &str, value: &str, deadline: i64) {
        self.connection.set::<_, _, ()>(key, value).unwrap();
        self.expire(key, deadline);
    }

    pub fn keys(&mut self) -> BTreeSet<String> {
        let mut cursor = 0u64;
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

    pub fn age(&mut self, key: &str, elapsed: i64) {
        let before = self.fields(key);
        let created = self.now() - elapsed;
        let absolute = (created + self.policy.absolute_ttl_seconds() as i64 * 1000)
            .min(number(&before, "ceiling_unix_ms"));
        let idle = self
            .policy
            .idle_ttl_seconds()
            .map(|ttl| (created + ttl as i64 * 1000).min(absolute));
        self.set(key, "created_at_unix_ms", created);
        self.set(key, "last_activity_unix_ms", created);
        self.set(key, "absolute_expires_at_unix_ms", absolute);
        self.set(key, "idle_expires_at_unix_ms", idle.unwrap_or(0));
        self.expire(key, idle.unwrap_or(absolute));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for key in &self.owned {
            let _ = self.connection.del::<_, ()>(key);
        }
    }
}

pub fn connect(url: &str) -> RedisSessionStore {
    RedisSessionStore::connect_with_timeouts(url, Duration::from_secs(2), Duration::from_secs(2))
        .unwrap()
}

pub fn digest(byte: u8) -> Sha256Digest {
    Sha256Digest::from_array([byte; 32])
}

pub fn key(namespace: &str, token: &str) -> String {
    format!(
        "identity:{namespace}:{}",
        RingSha256Hasher.hash_bytes(token.as_bytes()).to_hex()
    )
}

pub fn number(fields: &BTreeMap<String, String>, name: &str) -> i64 {
    fields[name].parse().unwrap()
}

pub fn token_shape(token: &str) {
    assert_eq!(token.len(), 43, "opaque token must be canonical base64url");
    let bytes = URL_SAFE_NO_PAD
        .decode(token)
        .expect("opaque token is not base64url");
    assert_eq!(bytes.len(), 32);
    assert!(
        URL_SAFE_NO_PAD.encode(bytes) == token,
        "opaque token is not canonical"
    );
}

fn time(connection: &mut redis::Connection) -> i64 {
    let (seconds, micros): (i64, i64) = redis::cmd("TIME").query(connection).unwrap();
    seconds * 1000 + micros / 1000
}

pub fn authentication(origin: &CertificateSessionProvenance) -> Value {
    json!({ "kind": "certificate", "provenance": {
        "principal": origin.principal, "auth_generation": origin.auth_generation,
        "binding_id": origin.binding_id, "deployment_id": origin.deployment_id,
        "trust_revision": origin.trust_revision,
        "root_fingerprint": origin.root_fingerprint.to_hex(),
        "leaf_fingerprint": origin.leaf_fingerprint.to_hex(), "crl_digest": origin.crl_digest.to_hex(),
        "valid_from_unix_seconds": origin.valid_from_unix_seconds,
        "valid_until_unix_seconds": origin.valid_until_unix_seconds,
    } })
}
