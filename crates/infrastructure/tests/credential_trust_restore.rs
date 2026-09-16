#[allow(dead_code)]
mod credential_trust_database_support;

use std::process::Command;

use application::credential_trust::{CredentialTrustExpectation, CredentialTrustStore};
use infrastructure::PostgresCredentialTrustStore;

use credential_trust_database_support::{materials, Clock, Database};

#[test]
fn dump_restore_preserves_deployment_public_material_history_and_audit_then_allows_successor() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let store = PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).unwrap();
    let first = store
        .publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(1, -100, 100),
        )
        .unwrap();
    let second = store
        .publish(
            CredentialTrustExpectation::Revision(first.revision),
            materials().inspection(2, -50, 100),
        )
        .unwrap();
    let before = db.snapshot();
    let temporary = tempfile::tempdir().unwrap();
    let dump = temporary.path().join("credential-trust.dump");
    let output = Command::new("pg_dump")
        .args([
            "--dbname",
            &db.url,
            "--schema",
            &db.schema,
            "--format=custom",
            "--file",
        ])
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    drop(store);
    db.client
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    let output = Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", &db.url])
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(db.snapshot(), before);
    db.reapply();
    assert_eq!(db.snapshot(), before);
    let store = PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).unwrap();
    let third = store
        .publish(
            CredentialTrustExpectation::Revision(second.revision),
            materials().inspection(u64::MAX, -25, 100),
        )
        .unwrap();
    assert_eq!(third.deployment_id, first.deployment_id);
    assert_eq!(third.inspection.crl_number, u64::MAX);
    assert_eq!(db.counts(), (1, 3, 3));
}

#[test]
fn sequence_trigger_and_digest_checks_work_with_an_empty_search_path() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let store = PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).unwrap();
    store
        .publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(1, -100, 100),
        )
        .unwrap();
    db.client.batch_execute("SET search_path=''").unwrap();
    db.client
        .batch_execute(&format!(
            "INSERT INTO {0}.participant_credential_trust_revisions
        SELECT deployment_id,2,crl_der,crl_digest,2,crl_this_update,crl_next_update,valid_from,
            valid_until,published_at_seconds,published_at_nanoseconds,published_by
        FROM {0}.participant_credential_trust_revisions WHERE revision=1",
            db.schema
        ))
        .unwrap();
    db.client
        .batch_execute(&format!("SET search_path={}", db.schema))
        .unwrap();
    assert_eq!(db.counts(), (1, 2, 1));
}
