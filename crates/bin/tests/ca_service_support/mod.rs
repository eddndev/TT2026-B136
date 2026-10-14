use std::{
    fs::OpenOptions,
    io::Write,
    net::TcpListener,
    os::unix::fs::OpenOptionsExt,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use application::identity::SecretProtector;
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::crypto::{
    PasswordHasher, RecoveryCodeGenerator, RecoveryCodeSet, TotpProvider, RECOVERY_CODE_COUNT,
};
use infrastructure::{
    AesGcmSecretProtector, Argon2idHasher, RandomRecoveryCodeGenerator, TotpRsProvider,
};
use reqwest::{blocking::Client, blocking::Response, Method, StatusCode, Url};
use serde_json::{json, Value};
use tempfile::TempDir;
use zeroize::Zeroizing;

use crate::ca_identity_support::Fixture;

pub struct Credentials {
    pub password: Zeroizing<String>,
    pub recovery: Vec<Zeroizing<String>>,
    secret: Zeroizing<Vec<u8>>,
    kek: Zeroizing<Vec<u8>>,
}

impl Credentials {
    pub fn enroll(fixture: &mut Fixture) -> Self {
        let password = Zeroizing::new(format!("CA-test-{}", uuid::Uuid::new_v4()));
        let mut kek = Zeroizing::new(Vec::with_capacity(32));
        kek.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
        kek.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
        let enrollment = TotpRsProvider::new()
            .enroll(&fixture.operator.email)
            .unwrap();
        let hasher = Argon2idHasher::new();
        let recovery = RandomRecoveryCodeGenerator
            .generate(RECOVERY_CODE_COUNT)
            .unwrap();
        let mut user = fixture.operator.clone();
        user.password_hash = hasher.hash(&password).unwrap();
        user.protected_totp_secret = AesGcmSecretProtector::new(kek.clone())
            .unwrap()
            .protect(user.id, &enrollment.secret)
            .unwrap();
        user.recovery_codes = RecoveryCodeSet::from_hashes(
            recovery
                .iter()
                .map(|code| hasher.hash(code).unwrap())
                .collect(),
        )
        .unwrap();
        let stored_codes = serde_json::to_value(&user.recovery_codes).unwrap();
        assert_eq!(
            fixture
                .admin()
                .execute(
                    "UPDATE qadra_ca.users SET password_hash=$1, protected_totp_secret=$2,
                 recovery_codes=$3 WHERE id=$4",
                    &[
                        &user.password_hash,
                        &user.protected_totp_secret,
                        &stored_codes,
                        &user.id.as_uuid()
                    ],
                )
                .unwrap(),
            1,
        );
        fixture.operator = user;
        Self {
            password,
            recovery,
            secret: enrollment.secret,
            kek,
        }
    }

    pub fn totp(&self) -> Zeroizing<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Zeroizing::new(
            TotpRsProvider::new()
                .current_code(&self.secret, now)
                .unwrap(),
        )
    }
}

pub fn redis_url() -> Zeroizing<String> {
    let raw = Zeroizing::new(
        std::env::var("CA_TEST_REDIS_URL")
            .or_else(|_| std::env::var("IDENTITY_TEST_REDIS_URL"))
            .expect("CA service tests require disposable CA_TEST_REDIS_URL"),
    );
    let parsed = Url::parse(&raw).expect("CA_TEST_REDIS_URL must be a Redis URL");
    assert_eq!(parsed.scheme(), "redis", "CA tests require local Redis");
    assert!(
        matches!(
            parsed.host_str(),
            Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
        ),
        "CA tests require disposable loopback Redis"
    );
    raw
}

pub struct Service {
    child: Option<Child>,
    client: Client,
    base_url: String,
    _private: TempDir,
}

impl Service {
    pub fn start(fixture: &Fixture, credentials: &Credentials, redis: &str) -> Self {
        Self::start_with_operations(fixture, credentials, redis, None)
    }

    pub fn start_with_operations(
        fixture: &Fixture,
        credentials: &Credentials,
        redis: &str,
        operations: Option<Value>,
    ) -> Self {
        let private = tempfile::tempdir().unwrap();
        let reserved = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = reserved.local_addr().unwrap();
        let config_path = private.path().join("private.json");
        let mut config = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&config_path)
            .unwrap();
        let mut settings = json!({
            "bind": address,
            "database_url": fixture.runtime,
            "database_name": fixture.database,
            "database_role": fixture.role,
            "redis_url": redis,
            "kek_base64": STANDARD.encode(&*credentials.kek),
            "operator_id": fixture.operator.id.as_uuid(),
            "operator_email": fixture.operator.email,
            "session_ttl_seconds": 300,
            "idle_ttl_seconds": 120,
        });
        if let Some(operations) = operations {
            settings["operations"] = operations;
        }
        serde_json::to_writer(&mut config, &settings).unwrap();
        config.flush().unwrap();
        drop(config);
        let client = Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        drop(reserved);
        let child = Command::new(env!("CARGO_BIN_EXE_qadra-ca"))
            .arg("--config")
            .arg(&config_path)
            .current_dir(private.path())
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("could not start the owned CA service process");
        let mut service = Self {
            child: Some(child),
            client,
            base_url: format!("http://{address}"),
            _private: private,
        };
        service.wait_until_healthy();
        service
    }

    fn wait_until_healthy(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            assert!(
                self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "owned CA service exited before becoming healthy"
            );
            if let Ok(response) = self
                .client
                .get(format!("{}/healthz", self.base_url))
                .timeout(Duration::from_millis(500))
                .send()
            {
                if response.status() == StatusCode::OK && response.text().unwrap() == "ok" {
                    return;
                }
            }
            assert!(
                Instant::now() < deadline,
                "owned CA service health deadline expired"
            );
            thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn post(&self, path: &str, body: Value) -> Response {
        self.client
            .post(format!("{}{path}", self.base_url))
            .json(&body)
            .send()
            .expect("CA HTTP request failed")
    }

    #[allow(dead_code)]
    pub fn post_authorized(&self, path: &str, token: &str, body: Value) -> Response {
        self.client
            .post(format!("{}{path}", self.base_url))
            .bearer_auth(token)
            .json(&body)
            .send()
            .expect("CA HTTP request failed")
    }

    pub fn bearer(&self, method: Method, path: &str, token: &str) -> Response {
        self.client
            .request(method, format!("{}{path}", self.base_url))
            .bearer_auth(token)
            .send()
            .expect("CA HTTP request failed")
    }

    pub fn login(&self, email: &str, password: &str) -> Zeroizing<String> {
        let mut response = response_json(
            self.post(
                "/api/v1/auth/login",
                json!({
                    "email": email, "password": password,
                }),
            ),
            StatusCode::OK,
        );
        assert!(response.get("access_token").is_none());
        assert!(response["expires_in_seconds"].as_u64().unwrap() > 0);
        take_secret(&mut response, "challenge_token")
    }

    #[allow(dead_code)]
    pub fn stop_with_sigterm(&mut self) -> ExitStatus {
        let child = self.child.as_mut().unwrap();
        assert!(
            child.try_wait().unwrap().is_none(),
            "owned CA service exited before SIGTERM"
        );
        assert!(
            Command::new("kill")
                .arg("-TERM")
                .arg(child.id().to_string())
                .status()
                .expect("could not signal the owned CA service")
                .success(),
            "SIGTERM was not delivered to the owned CA service"
        );
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                self.child = None;
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "owned CA service did not finish graceful shutdown"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn stop(&mut self) {
        let child = self.child.as_mut().unwrap();
        assert!(
            child.try_wait().unwrap().is_none(),
            "owned CA service exited unexpectedly"
        );
        child
            .kill()
            .expect("could not terminate the owned CA service process");
        child
            .wait()
            .expect("could not reap the owned CA service process");
        self.child = None;
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn response_json(response: Response, expected: StatusCode) -> Value {
    assert_eq!(response.status(), expected, "unexpected CA HTTP status");
    response.json().expect("CA HTTP response was not JSON")
}

pub fn denied(response: Response, code: &str) {
    let body = response_json(response, StatusCode::UNAUTHORIZED);
    assert!(
        body["error"]["code"].as_str() == Some(code),
        "unexpected CA error code"
    );
    assert!(body.get("access_token").is_none());
    assert!(body.get("challenge_token").is_none());
}

pub fn take_secret(value: &mut Value, field: &str) -> Zeroizing<String> {
    match value.as_object_mut().unwrap().remove(field) {
        Some(Value::String(secret)) if !secret.is_empty() => Zeroizing::new(secret),
        _ => panic!("expected a nonempty credential field in the CA response"),
    }
}
