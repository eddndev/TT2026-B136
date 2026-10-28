use crate::{case_administration_support::Fixture, credential_trust_database_support};
use application::credential_trust::{CredentialTrustExpectation, CredentialTrustStore};
use application::evidence_trust::CredentialTrustReader;
use infrastructure::credential_trust_postgres::{
    PostgresCredentialTrustReader, PostgresCredentialTrustStore,
};

use credential_trust_database_support::{materials, Clock};

fn audit_rows(db: &mut Fixture) -> i64 {
    db.admin
        .query_one("SELECT count(*) FROM audit_events", &[])
        .unwrap()
        .get(0)
}

#[test]
fn the_runtime_reader_follows_the_newest_publication_without_writing() {
    let Some(mut db) = Fixture::new() else {
        return;
    };
    let reader = PostgresCredentialTrustReader::open(&db.runtime_url).unwrap();
    assert_eq!(reader.current().unwrap(), None);

    let store =
        PostgresCredentialTrustStore::open(&db.admin_url, Clock::new(materials().at)).unwrap();
    let first = store
        .publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(1, -100, 100),
        )
        .unwrap();
    assert_eq!(reader.current().unwrap(), Some(first.clone()));

    let second = store
        .publish(
            CredentialTrustExpectation::Revision(first.revision),
            materials().inspection(2, -50, 150),
        )
        .unwrap();
    let audited = audit_rows(&mut db);
    assert_eq!(reader.current().unwrap(), Some(second));
    assert_eq!(audit_rows(&mut db), audited);
}

#[test]
fn opening_the_reader_requires_the_validated_runtime_schema() {
    let Some(db) = Fixture::old() else {
        return;
    };
    assert!(PostgresCredentialTrustReader::open(&db.runtime_url).is_err());
}
