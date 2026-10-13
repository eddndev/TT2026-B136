//! Owned PKI and public trust in an explicitly dedicated disposable PG cluster.
//! CA_OPERATIONS_TEST_ADMIN_URL must never name a shared or production cluster;
//! qadra and qadra_admin must be absent, and only this fixture's objects are dropped.
use postgres::{Client, NoTls};
use reqwest::Url;
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::PathBuf,
    process::{Command, Output, Stdio},
};
use tempfile::TempDir;
use zeroize::Zeroizing;

use crate::ca_identity_support::Fixture;

struct Trust {
    control: Client,
    database_created: bool,
    role_created: bool,
    password: Zeroizing<String>,
    port: u16,
}

impl Trust {
    fn new(identity: &Fixture, public: &Value) -> Self {
        let raw = Zeroizing::new(std::env::var("CA_OPERATIONS_TEST_ADMIN_URL").expect(
            "CA operations require CA_OPERATIONS_TEST_ADMIN_URL for a dedicated disposable cluster",
        ));
        let mut base = Url::parse(&raw).expect("CA operations admin URL is invalid");
        assert!(matches!(base.scheme(), "postgres" | "postgresql"));
        assert!(matches!(base.host_str(), Some("127.0.0.1" | "localhost")));
        let identity_url = Url::parse(&identity.runtime).unwrap();
        assert_eq!(
            base.host_str(),
            identity_url.host_str(),
            "CA fixtures require the same owned cluster"
        );
        assert_eq!(
            base.port(),
            identity_url.port(),
            "CA fixtures require the same owned cluster"
        );
        base.set_query(None);
        let mut control = Client::connect(base.as_str(), NoTls)
            .unwrap_or_else(|_| panic!("dedicated CA operations cluster is unavailable"));
        let occupied: bool = control
            .query_one(
                "SELECT
            EXISTS(SELECT 1 FROM pg_database WHERE datname='qadra') OR
            EXISTS(SELECT 1 FROM pg_roles WHERE rolname='qadra_admin')",
                &[],
            )
            .unwrap()
            .get(0);
        assert!(!occupied, "CA operations require qadra and qadra_admin to be absent before creating owned fixtures");
        let mut trust = Self {
            control,
            database_created: false,
            role_created: false,
            password: Zeroizing::new(uuid::Uuid::new_v4().simple().to_string()),
            port: base.port().unwrap_or(5432),
        };
        trust
            .control
            .batch_execute(&format!(
                "CREATE ROLE qadra_admin LOGIN NOINHERIT NOSUPERUSER
            NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{}'",
                &*trust.password
            ))
            .unwrap_or_else(|_| panic!("owned public trust role could not be created"));
        trust.role_created = true;
        trust
            .control
            .batch_execute("CREATE DATABASE qadra")
            .unwrap();
        trust.database_created = true;
        base.set_path("/qadra");
        let mut database = Client::connect(base.as_str(), NoTls).unwrap();
        database.batch_execute("CREATE TABLE participant_credential_authority (
            deployment_id UUID PRIMARY KEY, root_der BYTEA NOT NULL, root_fingerprint BYTEA NOT NULL);
            CREATE TABLE participant_credential_trust_revisions (
            deployment_id UUID NOT NULL, revision INTEGER NOT NULL, crl_der BYTEA NOT NULL,
            crl_digest BYTEA NOT NULL, crl_number NUMERIC(20) NOT NULL, crl_this_update BIGINT NOT NULL,
            crl_next_update BIGINT NOT NULL, valid_from BIGINT NOT NULL, valid_until BIGINT NOT NULL);
            GRANT USAGE ON SCHEMA public TO qadra_admin;
            GRANT SELECT ON participant_credential_authority, participant_credential_trust_revisions TO qadra_admin;")
            .unwrap();
        let deployment = uuid::Uuid::new_v4();
        database
            .execute(
                "INSERT INTO participant_credential_authority VALUES ($1,$2,$3)",
                &[
                    &deployment,
                    &unhex(&public["root_der"]),
                    &unhex(&public["root_fingerprint"]),
                ],
            )
            .unwrap();
        database
            .execute(
                "INSERT INTO participant_credential_trust_revisions
            VALUES ($1,1,$2,$3,$4::text::numeric,$5,$6,$7,$8)",
                &[
                    &deployment,
                    &unhex(&public["crl_der"]),
                    &unhex(&public["crl_digest"]),
                    &public["crl_number"].as_u64().unwrap().to_string(),
                    &public["crl_this_update"].as_i64().unwrap(),
                    &public["crl_next_update"].as_i64().unwrap(),
                    &public["valid_from"].as_i64().unwrap(),
                    &public["valid_until"].as_i64().unwrap(),
                ],
            )
            .unwrap();
        trust
    }
}

impl Drop for Trust {
    fn drop(&mut self) {
        if self.database_created {
            let _ = self
                .control
                .batch_execute("DROP DATABASE qadra WITH (FORCE)");
        }
        if self.role_created {
            let _ = self.control.batch_execute("DROP ROLE qadra_admin");
        }
    }
}

fn unhex(value: &Value) -> Vec<u8> {
    value
        .as_str()
        .unwrap()
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn run(command: &mut Command) -> Output {
    let output = command
        .stdin(Stdio::null())
        .output()
        .expect("owned PKI fixture command could not start");
    assert!(output.status.success(), "owned PKI fixture command failed");
    output
}

pub struct NativeCa {
    _private: TempDir,
    _trust: Trust,
    pub root: PathBuf,
    pub csr: String,
    pub fingerprint: String,
    pub holder_key: PathBuf,
    repository: PathBuf,
}

impl NativeCa {
    pub fn new(identity: &Fixture) -> Self {
        let private = tempfile::tempdir().unwrap();
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let root = private.path().join("qadra");
        let ca = root.join("data/ca");
        fs::create_dir_all(root.join("config")).unwrap();
        fs::create_dir_all(root.join("data/tmp")).unwrap();
        for script in ["init-ca.sh", "gen-crl.sh"] {
            run(Command::new("/bin/bash")
                .arg(repository.join("pki").join(script))
                .env("PKI_CA_DIR", &ca));
        }
        fs::write(root.join("deploy.lock"), []).unwrap();
        let holder = private.path().join("external-holder");
        fs::create_dir(&holder).unwrap();
        fs::set_permissions(&holder, fs::Permissions::from_mode(0o700)).unwrap();
        let holder_key = holder.join("holder.key.pem");
        let request = holder.join("holder.csr.pem");
        run(Command::new("openssl")
            .args([
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:3072",
                "-pkeyopt",
                "rsa_keygen_pubexp:65537",
                "-out",
            ])
            .arg(&holder_key));
        fs::set_permissions(&holder_key, fs::Permissions::from_mode(0o600)).unwrap();
        run(Command::new("openssl")
            .args(["req", "-new", "-sha256", "-key"])
            .arg(&holder_key)
            .arg("-out")
            .arg(&request)
            .args([
                "-subj",
                "/C=MX/O=Despacho Juridico Demo/CN=External HTTP Holder",
            ]));
        let code = "import sys,json;sys.path.insert(0,sys.argv[1]);import crl_material;print(json.dumps(crl_material.inspect(sys.argv[2],sys.argv[3])))";
        let inspection = run(Command::new("/usr/bin/python3")
            .args(["-I", "-B", "-S", "-c", code])
            .arg(repository.join("ops/deploy"))
            .arg(ca.join("ca.crt.pem"))
            .arg(ca.join("crl/crl.pem")));
        let public: Value = serde_json::from_slice(&inspection.stdout).unwrap();
        let trust = Trust::new(identity, &public);
        let mut settings = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(root.join("config/settings.json"))
            .unwrap();
        serde_json::to_writer(
            &mut settings,
            &json!({"postgres_port": trust.port,
            "admin_password": &*trust.password, "runtime_password": "unused-in-readonly-test",
            "redis_password": "unused-in-readonly-test", "redis_port": 1,
            "kek": "unused-in-readonly-test"}),
        )
        .unwrap();
        settings.flush().unwrap();
        Self {
            root,
            csr: fs::read_to_string(request).unwrap(),
            fingerprint: public["root_fingerprint"].as_str().unwrap().into(),
            holder_key,
            repository,
            _private: private,
            _trust: trust,
        }
    }

    pub fn operations(&self) -> Value {
        json!({"python": "/usr/bin/python3", "worker": self.repository.join("ops/ca/command.py"),
            "root": self.root, "inspector": env!("CARGO_BIN_EXE_qadra-ca"),
            "cli": env!("CARGO_BIN_EXE_despacho-cli"), "scripts": self.repository.join("pki"),
            "expected_root": self.fingerprint})
    }

    pub fn command(&self, purpose: &str) -> Value {
        json!({"action": "issue", "operation_id": uuid::Uuid::new_v4(), "csr_pem": self.csr,
            "purpose": purpose, "holder": "External HTTP Holder", "reference": "owned-http-case",
            "criterion": "External holder and proof reviewed", "expected_root_fingerprint": self.fingerprint})
    }

    pub fn ca_file(&self, name: &str) -> PathBuf {
        self.root.join("data/ca").join(name)
    }

    pub fn assert_signed_by_root(&self, certificate: &str) {
        let path = self.root.join("checked-public-leaf.pem");
        fs::write(&path, certificate).unwrap();
        run(Command::new("openssl")
            .args(["verify", "-CAfile"])
            .arg(self.ca_file("ca.crt.pem"))
            .arg(path));
    }
}
