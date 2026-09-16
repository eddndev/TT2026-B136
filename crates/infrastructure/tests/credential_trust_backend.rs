#[allow(dead_code)]
mod credential_trust_database_support;

use application::credential_trust::{CredentialTrustExpectation, CredentialTrustStore};
use application::ApplicationError;
use infrastructure::credential_trust_postgres::PostgresCredentialTrustStore;

use credential_trust_database_support::{materials, Clock, Database};

#[test]
fn first_publication_and_successor_preserve_public_bytes_and_database_actor() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let clock = Clock::new(materials().at);
    let store = PostgresCredentialTrustStore::open(&db.url, clock.clone()).unwrap();
    let first_material = materials().inspection(1, -100, 100);
    let first = store
        .publish(CredentialTrustExpectation::Absent, first_material.clone())
        .unwrap();
    assert_eq!(first.revision.get(), 1);
    assert_eq!(first.inspection, first_material);
    assert_eq!(first.published_at.unix_timestamp(), materials().at);
    assert_eq!(first.published_by, db.session_user());
    let second_material = materials().inspection(2, -50, 150);
    let second = store
        .publish(
            CredentialTrustExpectation::Revision(first.revision),
            second_material.clone(),
        )
        .unwrap();
    assert_eq!(second.revision.get(), 2);
    assert_eq!(second.deployment_id, first.deployment_id);
    assert_eq!(second.inspection, second_material);
    assert_eq!(db.counts(), (1, 2, 2));
    let before = db.snapshot();
    db.reapply();
    assert_eq!(db.snapshot(), before);
    let events = db
        .client
        .query(
            "SELECT actor,action,resource FROM audit_events ORDER BY sequence",
            &[],
        )
        .unwrap();
    for row in events {
        assert_eq!(
            row.get::<_, String>(0),
            format!("database-admin:{}", first.published_by)
        );
        assert_eq!(
            row.get::<_, String>(1),
            "participant.credential_trust_published"
        );
        assert!(row
            .get::<_, String>(2)
            .contains(&first.deployment_id.to_string()));
    }
}

#[test]
fn stale_expected_and_non_monotonic_crls_leave_all_rows_and_audit_unchanged() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let store = PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).unwrap();
    let first = store
        .publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(10, -50, 100),
        )
        .unwrap();
    let before = db.snapshot();
    assert!(matches!(
        store.publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(11, -10, 100)
        ),
        Err(ApplicationError::CredentialTrustRevisionConflict)
    ));
    for material in [
        materials().inspection(10, -40, 100),
        materials().inspection(9, -40, 100),
        materials().inspection(11, -60, 100),
    ] {
        assert!(store
            .publish(
                CredentialTrustExpectation::Revision(first.revision),
                material
            )
            .is_err());
        assert_eq!(db.snapshot(), before);
    }
}

#[test]
fn audit_failure_rolls_back_the_authority_and_its_initial_revision() {
    let Some(mut db) = Database::new() else {
        return;
    };
    db.client.batch_execute("ALTER TABLE audit_events ADD CONSTRAINT reject_publication CHECK(action<>'participant.credential_trust_published')").unwrap();
    let store = PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).unwrap();
    assert!(store
        .publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(1, -100, 100)
        )
        .is_err());
    assert_eq!(db.counts(), (0, 0, 0));
}

#[test]
fn expired_or_corrupt_inspection_cannot_create_trust() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let store =
        PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at + 101)).unwrap();
    assert!(store
        .publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(1, -100, 100)
        )
        .is_err());
    assert_eq!(db.counts(), (0, 0, 0));
    let store = PostgresCredentialTrustStore::open(&db.url, Clock::new(materials().at)).unwrap();
    let mut damaged = materials().inspection(1, -100, 100);
    damaged.crl_der[0] ^= 1;
    assert!(store
        .publish(CredentialTrustExpectation::Absent, damaged)
        .is_err());
    assert_eq!(db.counts(), (0, 0, 0));
}

#[test]
fn successor_audit_failure_and_root_substitution_preserve_all_captured_history() {
    use domain::crypto::DocumentHasher;
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
    let before = db.snapshot();
    let mut changed_root = materials().inspection(2, -50, 100);
    changed_root.root_der[0] ^= 1;
    changed_root.root_fingerprint =
        infrastructure::RingSha256Hasher.hash_bytes(&changed_root.root_der);
    assert!(matches!(
        store.publish(
            CredentialTrustExpectation::Revision(first.revision),
            changed_root
        ),
        Err(ApplicationError::CredentialTrustChanged)
    ));
    assert_eq!(db.snapshot(), before);
    db.client
        .batch_execute("ALTER TABLE audit_events ADD CONSTRAINT reject_successor CHECK(sequence<1)")
        .unwrap();
    assert!(store
        .publish(
            CredentialTrustExpectation::Revision(first.revision),
            materials().inspection(2, -50, 100)
        )
        .is_err());
    assert_eq!(db.snapshot(), before);
}

#[test]
fn publication_provenance_uses_session_user_even_when_a_database_role_is_selected() {
    let Some(mut db) = Database::new() else {
        return;
    };
    db.role("GRANT SELECT,INSERT ON participant_credential_authority,participant_credential_trust_revisions,audit_events TO {role}");
    let session_user = db.session_user();
    let mut url = reqwest::Url::parse(&db.url).unwrap();
    url.query_pairs_mut().clear().append_pair(
        "options",
        &format!("-csearch_path={} -crole={}", db.schema, db.roles[0]),
    );
    let url = url.to_string().replace('+', "%20");
    let store = PostgresCredentialTrustStore::open(&url, Clock::new(materials().at)).unwrap();
    let result = store
        .publish(
            CredentialTrustExpectation::Absent,
            materials().inspection(1, -100, 100),
        )
        .unwrap();
    assert_eq!(result.published_by, session_user);
    assert_ne!(result.published_by, db.roles[0]);
}
