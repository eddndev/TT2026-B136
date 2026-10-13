use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use domain::crypto::TotpProvider;
use infrastructure::{decode_base32_secret, TotpRsProvider};
use serde_json::{json, Value};
use zeroize::Zeroizing;

use crate::ca_identity_support::{search_path, Fixture};

pub struct Harness {
    pub db: Fixture,
    pub directory: tempfile::TempDir,
    pub setup: PathBuf,
    pub enrollment: PathBuf,
    pub config: Value,
    pub password: Zeroizing<String>,
    pub kek: Zeroizing<Vec<u8>>,
}

impl Harness {
    pub fn new() -> Self {
        let mut db = Fixture::new();
        db.admin()
            .batch_execute("DELETE FROM qadra_ca.users")
            .unwrap();
        let role: String = db
            .admin()
            .query_one("SELECT current_user::text", &[])
            .unwrap()
            .get(0);
        let mut admin =
            reqwest::Url::parse(&std::env::var("IDENTITY_TEST_DATABASE_URL").unwrap()).unwrap();
        admin.set_path(&format!("/{}", db.database));
        let redis = std::env::var("CA_TEST_REDIS_URL")
            .or_else(|_| std::env::var("IDENTITY_TEST_REDIS_URL"))
            .expect("CA enrollment tests require disposable CA_TEST_REDIS_URL");
        let directory = tempfile::tempdir().unwrap();
        let setup = directory.path().join("setup.json");
        let enrollment = directory.path().join("enrollment.json");
        let kek = Zeroizing::new(vec![43; 32]);
        let config = json!({
            "database_name": db.database,
            "operator_email": db.operator.email,
            "kek_base64": STANDARD.encode(&*kek),
            "admin": {"database_url": search_path(admin.as_str(), "qadra_ca,pg_temp"),
                "database_role": role},
            "runtime": {"database_url": db.runtime, "database_role": db.role,
                "redis_url": redis},
        });
        private_json(&setup, &config);
        Self {
            db,
            directory,
            setup,
            enrollment,
            config,
            password: Zeroizing::new(format!("CA-enrollment-{}", uuid::Uuid::new_v4())),
            kek,
        }
    }

    pub fn spawn(&self) -> EnrollmentChild {
        self.spawn_with_input(Stdio::piped())
    }

    pub fn spawn_with_input(&self, stdin: Stdio) -> EnrollmentChild {
        let suffix = uuid::Uuid::new_v4();
        let stdout = self.directory.path().join(format!("stdout-{suffix}"));
        let stderr = self.directory.path().join(format!("stderr-{suffix}"));
        let child = std::process::Command::new(env!("CARGO_BIN_EXE_qadra-ca"))
            .args(["enroll-operator", "--setup-config"])
            .arg(&self.setup)
            .arg("--enrollment-out")
            .arg(&self.enrollment)
            .current_dir(self.directory.path())
            .env_clear()
            .stdin(stdin)
            .stdout(private_file(&stdout))
            .stderr(private_file(&stderr))
            .spawn()
            .expect("owned CA enrollment process could not start");
        EnrollmentChild {
            child,
            stdout,
            stderr,
        }
    }

    pub fn rewrite_config(&self, config: &Value) {
        let bytes = serde_json::to_vec(config).unwrap();
        fs::write(&self.setup, bytes).unwrap();
        fs::set_permissions(&self.setup, fs::Permissions::from_mode(0o600)).unwrap();
    }

    pub fn bundle(&self, child: &mut EnrollmentChild) -> Value {
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            assert!(
                child.child.try_wait().unwrap().is_none(),
                "CA enrollment exited before delivering its private bundle"
            );
            if let Ok(bytes) = fs::read(&self.enrollment) {
                if let Ok(bundle) = serde_json::from_slice::<Value>(&bytes) {
                    if bundle.get("totp_secret_base32").is_some() {
                        return bundle;
                    }
                }
            }
            assert!(
                Instant::now() < deadline,
                "CA enrollment bundle deadline expired"
            );
            thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn assert_empty(&mut self) {
        let row = self
            .db
            .admin()
            .query_one(
                "SELECT (SELECT count(*) FROM qadra_ca.users),
                    (SELECT count(*) FROM qadra_ca.audit_events)",
                &[],
            )
            .unwrap();
        assert_eq!(row.get::<_, i64>(0), 0);
        assert_eq!(row.get::<_, i64>(1), 0);
        self.db.assert_office_unchanged();
    }

    pub fn assert_private_output(&self, result: &ResultOutput, bundle: Option<&Value>) {
        for bytes in [&result.stdout, &result.stderr] {
            let output = String::from_utf8_lossy(bytes);
            assert!(
                !output.contains(self.password.as_str()),
                "password leaked from enrollment"
            );
            assert!(
                !output.contains(&STANDARD.encode(&*self.kek)),
                "KEK leaked from enrollment"
            );
            if let Some(bundle) = bundle {
                for field in ["totp_secret_base32", "otpauth_uri"] {
                    assert!(
                        !output.contains(bundle[field].as_str().unwrap()),
                        "TOTP enrollment material leaked to process output"
                    );
                }
                for code in bundle["recovery_codes"].as_array().unwrap() {
                    assert!(
                        !output.contains(code.as_str().unwrap()),
                        "recovery code leaked"
                    );
                }
            }
        }
    }
}

fn private_file(path: &Path) -> File {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap()
}

pub fn private_json(path: &Path, value: &Value) {
    serde_json::to_writer(private_file(path), value).unwrap();
}

pub fn totp(bundle: &Value) -> Zeroizing<String> {
    let secret = decode_base32_secret(bundle["totp_secret_base32"].as_str().unwrap()).unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    Zeroizing::new(TotpRsProvider::new().current_code(&secret, now).unwrap())
}

pub struct EnrollmentChild {
    pub child: Child,
    stdout: PathBuf,
    stderr: PathBuf,
}

pub struct ResultOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl EnrollmentChild {
    pub fn line(&mut self, value: &str) {
        let input = self.child.stdin.as_mut().unwrap();
        input
            .write_all(value.as_bytes())
            .expect("owned enrollment input closed unexpectedly");
        input.write_all(b"\n").unwrap();
        input.flush().unwrap();
    }

    pub fn finish(&mut self) -> ResultOutput {
        self.child.stdin.take();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return ResultOutput {
                    status,
                    stdout: fs::read(&self.stdout).unwrap(),
                    stderr: fs::read(&self.stderr).unwrap(),
                };
            }
            assert!(
                Instant::now() < deadline,
                "owned enrollment process did not finish"
            );
            thread::sleep(Duration::from_millis(25));
        }
    }
}

impl Drop for EnrollmentChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
