use std::collections::BTreeMap;

use application::identity::{Principal, SessionIdentity, SessionPolicy};
use domain::crypto::DocumentHasher;
use domain::identity::{Role, UserId};
use infrastructure::{RedisSessionStore, RingSha256Hasher};
use redis::Commands;

pub struct Fixture {
    pub url: String,
    pub store: RedisSessionStore,
    pub connection: redis::Connection,
    pub token: String,
    pub key: String,
    pub identity: SessionIdentity,
    pub policy: SessionPolicy,
}

impl Fixture {
    pub fn new(idle_seconds: Option<u64>) -> Option<Self> {
        let Ok(url) = std::env::var("IDENTITY_TEST_REDIS_URL") else {
            eprintln!("skipping Redis lifetime test: IDENTITY_TEST_REDIS_URL is unset");
            return None;
        };
        let store = RedisSessionStore::connect(&url).unwrap();
        let connection = redis::Client::open(url.as_str())
            .unwrap()
            .get_connection()
            .unwrap();
        let token = uuid::Uuid::new_v4().to_string();
        let key = session_key(&token);
        Some(Self {
            url,
            store,
            connection,
            token,
            key,
            identity: SessionIdentity {
                principal: Principal {
                    id: UserId::new(),
                    email: "owner@example.test".into(),
                    role: Role::Owner,
                },
                auth_generation: 0,
            },
            policy: SessionPolicy::new(120, idle_seconds).unwrap(),
        })
    }

    pub fn now(&mut self) -> i64 {
        let (seconds, microseconds): (i64, i64) =
            redis::cmd("TIME").query(&mut self.connection).unwrap();
        seconds * 1_000 + microseconds / 1_000
    }

    pub fn seed(&mut self, elapsed_ms: i64) {
        let created = self.now() - elapsed_ms;
        let absolute_ttl = self.policy.absolute_ttl_seconds() as i64 * 1_000;
        let idle_ttl = self.policy.idle_ttl_seconds().unwrap_or(0) as i64 * 1_000;
        let absolute = created + absolute_ttl;
        let idle = if idle_ttl == 0 { 0 } else { created + idle_ttl };
        let fields = [
            ("version", "1".to_string()),
            (
                "identity_json",
                serde_json::to_string(&self.identity).unwrap(),
            ),
            ("absolute_ttl_ms", absolute_ttl.to_string()),
            ("idle_ttl_ms", idle_ttl.to_string()),
            ("created_at_unix_ms", created.to_string()),
            ("last_activity_unix_ms", created.to_string()),
            ("absolute_expires_at_unix_ms", absolute.to_string()),
            ("idle_expires_at_unix_ms", idle.to_string()),
        ];
        self.connection.del::<_, ()>(&self.key).unwrap();
        self.connection
            .hset_multiple::<_, _, _, ()>(&self.key, &fields)
            .unwrap();
        self.expire_at(if idle == 0 { absolute } else { idle });
    }

    pub fn raw(&mut self) -> BTreeMap<String, String> {
        self.connection.hgetall(&self.key).unwrap()
    }

    pub fn set(&mut self, field: &str, value: impl redis::ToRedisArgs) {
        self.connection
            .hset::<_, _, _, ()>(&self.key, field, value)
            .unwrap();
    }

    pub fn expire_at(&mut self, deadline: i64) {
        redis::cmd("PEXPIREAT")
            .arg(&self.key)
            .arg(deadline)
            .query::<()>(&mut self.connection)
            .unwrap();
    }

    pub fn exists(&mut self) -> bool {
        self.connection.exists(&self.key).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.connection.del::<_, ()>(&self.key);
    }
}

pub fn session_key(token: &str) -> String {
    format!(
        "identity:session:{}",
        RingSha256Hasher.hash_bytes(token.as_bytes()).to_hex()
    )
}

pub fn number(fields: &BTreeMap<String, String>, name: &str) -> i64 {
    fields[name].parse().unwrap()
}
