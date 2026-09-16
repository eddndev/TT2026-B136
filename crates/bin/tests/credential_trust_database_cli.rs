//! Exercises the administrative command against an explicitly disposable cluster.

use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;
use uuid::Uuid;

struct Database {
    base: String,
    url: String,
    runtime_url: String,
    name: String,
    role: String,
}
impl Database {
    fn new() -> Option<Self> {
        let base = std::env::var("IDENTITY_TEST_DATABASE_URL").ok()?;
        let nonce = Uuid::new_v4().simple().to_string();
        let name = format!("tt_cli_trust_{nonce}");
        let role = format!("tt_cli_publisher_test_{nonce}");
        let server = base
            .rsplit_once('/')
            .expect("test URL must be a PostgreSQL URI")
            .0;
        let host = server
            .rsplit_once('@')
            .expect("test URL must include isolated credentials")
            .1;
        let url = format!("{server}/{name}");
        let runtime_url = format!("postgresql://{role}:{nonce}@{host}/{name}");
        sql(&base, &format!("CREATE DATABASE {name}"));
        sql(
            &base,
            &format!(
                "CREATE ROLE {role} LOGIN PASSWORD '{nonce}' NOSUPERUSER NOCREATEDB NOCREATEROLE"
            ),
        );
        Some(Self {
            base,
            url,
            runtime_url,
            name,
            role,
        })
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        for query in [
            format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", self.name),
            format!("DROP ROLE IF EXISTS {}", self.role),
        ] {
            let _ = Command::new("psql")
                .arg("--dbname")
                .arg(&self.base)
                .args(["-X", "-v", "ON_ERROR_STOP=1", "-c", &query])
                .output();
        }
    }
}
fn sql(url: &str, query: &str) -> String {
    let output = Command::new("psql")
        .arg("--dbname")
        .arg(url)
        .args(["-X", "-qAt", "-v", "ON_ERROR_STOP=1", "-c", query])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "isolated SQL command failed: {}",
        String::from_utf8_lossy(&output.stderr).replace(url, "[isolated test database]")
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
fn cli(url: &str, dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .current_dir(dir)
        .env("DATABASE_URL", url)
        .args(args)
        .output()
        .unwrap()
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn pki(dir: &TempDir, args: &[&str]) {
    let output = Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .current_dir(dir.path())
        .env("PKI_CA_DIR", dir.path().join("ca"))
        .args(["pki", "--scripts-dir"])
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../pki"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "PKI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn publish(db: &str, dir: &TempDir, expected: &str) -> Output {
    cli(
        db,
        dir.path(),
        &[
            "credential-trust",
            "publish",
            "--root-cert",
            "ca/ca.crt.pem",
            "--crl",
            "ca/crl/crl.pem",
            "--expected-revision",
            expected,
            "--json",
        ],
    )
}

#[test]
fn administrator_publishes_successors_while_runtime_replay_and_bad_material_leave_no_trace() {
    let Some(db) = Database::new() else {
        eprintln!("credential trust CLI database test not exercised: IDENTITY_TEST_DATABASE_URL is absent");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    success(cli(
        &db.url,
        dir.path(),
        &["database", "migrate", "--runtime-role", &db.role, "--json"],
    ));
    pki(&dir, &["init-ca"]);
    pki(&dir, &["gen-crl"]);
    let first = success(publish(&db.url, &dir, "0"));
    assert_eq!(first["revision"], 1);
    assert!(first["crl_number"].is_string());
    assert_eq!(
        sql(
            &db.url,
            "SELECT count(*) FROM participant_credential_trust_revisions"
        ),
        "1"
    );
    assert_eq!(sql(&db.url,"SELECT count(*) FROM audit_events WHERE action='participant.credential_trust_published'"),"1");
    let replay = publish(&db.url, &dir, "0");
    assert!(!replay.status.success());
    assert!(String::from_utf8_lossy(&replay.stderr).contains("revision changed"));
    pki(&dir, &["gen-crl"]);
    let denied = publish(&db.runtime_url, &dir, "1");
    assert!(!denied.status.success());
    assert_eq!(
        sql(
            &db.url,
            "SELECT count(*) FROM participant_credential_trust_revisions"
        ),
        "1"
    );
    let second = success(publish(&db.url, &dir, "1"));
    assert_eq!(second["revision"], 2);
    assert_eq!(second["deployment_id"], first["deployment_id"]);
    assert_eq!(second["root_fingerprint"], first["root_fingerprint"]);
    assert_ne!(second["crl_number"], first["crl_number"]);
    assert_eq!(second["published_by"], sql(&db.url, "SELECT SESSION_USER"));
    let repeated = publish(&db.url, &dir, "2");
    assert!(!repeated.status.success());
    std::fs::write(dir.path().join("ca/crl/crl.pem"), b"invalid public CRL").unwrap();
    assert!(!publish(&db.url, &dir, "2").status.success());
    assert_eq!(
        sql(
            &db.url,
            "SELECT count(*) FROM participant_credential_trust_revisions"
        ),
        "2"
    );
    assert_eq!(sql(&db.url,"SELECT count(*) FROM audit_events WHERE action='participant.credential_trust_published'"),"2");
}
