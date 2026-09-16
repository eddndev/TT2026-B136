#[allow(dead_code)]
mod credential_trust_database_support;

use application::credential_trust::{CredentialTrustExpectation, CredentialTrustStore};
use infrastructure::PostgresCredentialTrustStore;

use credential_trust_database_support::{materials, Clock, Database};

#[test]
fn opening_missing_or_weakened_schema_fails_without_repairing_it() {
    for change in [
        "DROP TRIGGER credential_trust_sequence ON participant_credential_trust_revisions",
        "ALTER TABLE participant_credential_trust_revisions DISABLE TRIGGER credential_trust_immutable",
        "ALTER TABLE participant_credential_authority DISABLE TRIGGER ALL",
        "ALTER TABLE participant_credential_trust_revisions DROP CONSTRAINT credential_trust_digest",
        "ALTER TABLE participant_credential_trust_revisions ALTER COLUMN published_by DROP NOT NULL",
        "ALTER TABLE participant_credential_authority DROP CONSTRAINT credential_authority_initial_fk",
        "ALTER TABLE participant_credential_trust_revisions ALTER COLUMN crl_number TYPE numeric(21,0)",
        "ALTER TABLE participant_credential_authority DROP CONSTRAINT credential_authority_singleton_unique",
    ] {
        let Some(mut db) = Database::new() else { return };
        db.client.batch_execute(change).unwrap();
        assert!(PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).is_err(), "{change}");
    }
}

#[test]
fn authority_requires_its_first_revision_and_cannot_be_changed_or_truncated() {
    let Some(mut db) = Database::new() else {
        return;
    };
    assert!(db
        .client
        .batch_execute(
            "INSERT INTO participant_credential_authority(deployment_id,root_der,root_fingerprint)
        VALUES('00000000-0000-0000-0000-000000000001','a',sha256('a'))"
        )
        .is_err());
    assert_eq!(db.counts(), (0, 0, 0));
    let store = PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).unwrap();
    store
        .publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(1, -100, 100),
        )
        .unwrap();
    let before = db.snapshot();
    for mutation in [
        "UPDATE participant_credential_authority SET singleton=singleton",
        "DELETE FROM participant_credential_authority",
        "TRUNCATE participant_credential_authority,participant_credential_trust_revisions",
        "UPDATE participant_credential_trust_revisions SET crl_number=crl_number",
        "DELETE FROM participant_credential_trust_revisions",
        "TRUNCATE participant_credential_trust_revisions,participant_credential_authority",
    ] {
        assert!(db.client.batch_execute(mutation).is_err(), "{mutation}");
        assert_eq!(db.snapshot(), before);
    }
}

#[test]
fn direct_sql_cannot_skip_revisions_move_crls_backwards_or_use_repeatable_read() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let store = PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).unwrap();
    store
        .publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(2, -100, 100),
        )
        .unwrap();
    let before = db.snapshot();
    let insert = "INSERT INTO participant_credential_trust_revisions
        SELECT deployment_id,REVISION,crl_der,crl_digest,NUMBER,crl_this_update,crl_next_update,
            valid_from,valid_until,published_at_seconds,published_at_nanoseconds,published_by
        FROM participant_credential_trust_revisions WHERE revision=1";
    for (revision, number) in [(3, 3), (2, 2), (2, 1)] {
        assert!(db
            .client
            .batch_execute(
                &insert
                    .replace("REVISION", &revision.to_string())
                    .replace("NUMBER", &number.to_string())
            )
            .is_err());
        assert_eq!(db.snapshot(), before);
    }
    let mut tx = db
        .client
        .build_transaction()
        .isolation_level(postgres::IsolationLevel::RepeatableRead)
        .start()
        .unwrap();
    assert!(tx
        .batch_execute(&insert.replace("REVISION", "2").replace("NUMBER", "3"))
        .is_err());
    tx.rollback().unwrap();
    assert_eq!(db.snapshot(), before);
}

#[test]
fn opening_detects_corruption_even_after_disabled_guards_are_reenabled() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let store = PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).unwrap();
    let first = store
        .publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(2, -100, 100),
        )
        .unwrap();
    store
        .publish(
            CredentialTrustExpectation::Revision(first.revision),
            materials().inspection(3, -50, 100),
        )
        .unwrap();
    db.client.batch_execute("ALTER TABLE participant_credential_trust_revisions DISABLE TRIGGER credential_trust_immutable;
        UPDATE participant_credential_trust_revisions SET crl_number=1 WHERE revision=2;
        ALTER TABLE participant_credential_trust_revisions ENABLE TRIGGER credential_trust_immutable").unwrap();
    assert!(PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).is_err());
}
