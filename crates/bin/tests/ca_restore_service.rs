//! Native CA identity restoration preserves credentials and requires fresh MFA.
#![cfg(target_os = "linux")]

#[allow(dead_code)]
#[path = "../../infrastructure/tests/ca_identity_cases/support.rs"]
mod ca_identity_support;
#[allow(dead_code)]
mod ca_service_support;

use std::{path::Path, process::Command};

use domain::audit::{verify_chain, AuditLog, ChainVerification};
use infrastructure::RingSha256Hasher;
use reqwest::{Method, StatusCode, Url};
use serde_json::{json, Value};

use ca_identity_support::Fixture;
use ca_service_support::{denied, response_json, take_secret, Credentials, Service};

#[test]
fn restore_keeps_operator_audit_and_consumed_recovery_without_office_authority() {
    let redis = ca_service_support::redis_url();
    let mut fixture = Fixture::new();
    let credentials = Credentials::enroll(&mut fixture);
    let mut service = Service::start(&fixture, &credentials, &redis);
    let token = recover(&service, &fixture, &credentials, 0);
    assert_eq!(
        service
            .bearer(Method::POST, "/api/v1/auth/logout", &token)
            .status(),
        StatusCode::NO_CONTENT
    );
    service.stop();
    let before = snapshot(&mut fixture);
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("ca.dump");
    postgres_command("pg_dump", &fixture.database, &dump, false);
    fixture
        .admin()
        .batch_execute("DROP SCHEMA qadra_ca CASCADE")
        .unwrap();
    assert!(fixture.open().is_err());
    postgres_command("pg_restore", &fixture.database, &dump, true);
    assert!(
        snapshot(&mut fixture) == before,
        "CA restore changed account or audit bytes"
    );
    fixture.assert_office_unchanged();

    let mut restored = Service::start(&fixture, &credentials, &redis);
    denied(
        restored.bearer(Method::GET, "/api/v1/auth/me", &token),
        "invalid_session",
    );
    let challenge = restored.login(&fixture.operator.email, &credentials.password);
    denied(
        restored.bearer(Method::GET, "/api/v1/auth/me", &challenge),
        "invalid_session",
    );
    denied(
        restored.post(
            "/api/v1/auth/mfa/recovery",
            json!({
                "challenge_token": &*challenge, "code": &*credentials.recovery[0],
            }),
        ),
        "mfa_rejected",
    );
    let fresh = recover(&restored, &fixture, &credentials, 1);
    let me = response_json(
        restored.bearer(Method::GET, "/api/v1/auth/me", &fresh),
        StatusCode::OK,
    );
    assert_eq!(me["id"], fixture.operator.id.to_string());
    assert_eq!(me["email"], fixture.operator.email);
    assert!(me.get("role").is_none());
    assert_eq!(
        restored
            .bearer(Method::GET, "/api/v1/cases", &fresh)
            .status(),
        StatusCode::NOT_FOUND
    );
    restored.stop();
    let (_, audit) = fixture.open().unwrap();
    let entries = audit.load_all().unwrap();
    assert_eq!(
        verify_chain(&RingSha256Hasher, &entries).unwrap(),
        ChainVerification::Valid {
            entries: entries.len()
        }
    );
}

fn recover(
    service: &Service,
    fixture: &Fixture,
    credentials: &Credentials,
    index: usize,
) -> zeroize::Zeroizing<String> {
    let challenge = service.login(&fixture.operator.email, &credentials.password);
    let mut response = response_json(
        service.post(
            "/api/v1/auth/mfa/recovery",
            json!({
                "challenge_token": &*challenge, "code": &*credentials.recovery[index],
            }),
        ),
        StatusCode::OK,
    );
    take_secret(&mut response, "access_token")
}

fn snapshot(fixture: &mut Fixture) -> Vec<Value> {
    ["users", "audit_events"]
        .into_iter()
        .map(|table| {
            fixture
                .admin()
                .query_one(
                    &format!(
            "SELECT COALESCE(jsonb_agg(to_jsonb(row) ORDER BY to_jsonb(row)::text),'[]'::jsonb)
             FROM qadra_ca.{table} AS row"
        ),
                    &[],
                )
                .unwrap()
                .get(0)
        })
        .collect()
}

fn postgres_command(program: &str, database: &str, dump: &Path, restore: bool) {
    let raw = std::env::var("IDENTITY_TEST_DATABASE_URL").unwrap();
    let url = Url::parse(&raw).unwrap();
    let mut command = Command::new(program);
    command
        .env("PGHOST", url.host_str().unwrap())
        .env("PGPORT", url.port().unwrap_or(5432).to_string())
        .env("PGUSER", url.username())
        .env("PGPASSWORD", url.password().unwrap_or(""))
        .env("PGDATABASE", database)
        .env_remove("PGOPTIONS");
    if restore {
        command
            .args([
                "--exit-on-error",
                "--clean",
                "--if-exists",
                "--dbname",
                database,
            ])
            .arg(dump);
    } else {
        command.args(["--format=custom", "--file"]).arg(dump);
    }
    let output = command
        .output()
        .expect("native PostgreSQL tools are required");
    assert!(
        output.status.success(),
        "native CA SQL backup/restore failed"
    );
}
