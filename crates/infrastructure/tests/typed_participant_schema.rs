mod case_administration_support;
use case_administration_support::Fixture;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;

#[test]
fn typed_schema_adds_empty_identity_and_revision_families_without_fabricating_legacy() {
    let Some(mut db) = Fixture::new() else { return };
    for name in [
        "case_subjects",
        "case_subject_revisions",
        "case_participant_typed_revisions",
        "subject_identity_reviews",
        "participant_identity_reviews",
        "participant_credential_evidence",
    ] {
        assert!(
            db.admin
                .query_one("SELECT to_regclass($1) IS NOT NULL", &[&name])
                .unwrap()
                .get::<_, bool>(0),
            "{name}"
        );
    }
    let before = db.snapshot();
    db.migrate();
    assert_eq!(before, db.snapshot());
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_subjects", &[])
            .unwrap()
            .get::<_, i64>(0),
        0
    );
}

#[test]
fn runtime_startup_rejects_missing_typed_guards_without_repairing_schema() {
    for alteration in [
        "DROP TRIGGER typed_sequence ON case_participant_typed_revisions",
        "ALTER TABLE case_subject_revisions DISABLE TRIGGER subject_sequence",
        "DROP TRIGGER subject_root_complete ON case_subjects",
        "DROP TRIGGER typed_root_complete ON case_participants",
        "DROP TRIGGER participant_review_guard ON participant_identity_reviews",
        "DROP TRIGGER participant_credential_guard ON participant_credential_evidence",
        "ALTER TABLE case_subject_revisions DROP CONSTRAINT subject_projection",
        "ALTER TABLE case_subject_revisions ALTER COLUMN values_view DROP EXPRESSION",
        "ALTER TABLE participant_credential_evidence ALTER COLUMN signature DROP NOT NULL",
        "ALTER TABLE case_participant_typed_revisions DROP CONSTRAINT typed_credential_origin_fk",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
        db.admin.batch_execute(alteration).unwrap();
        assert!(
            PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
            "{alteration}"
        );
    }
}

#[test]
fn startup_requires_named_canonical_checks_even_when_the_number_of_checks_matches() {
    let Some(mut db) = Fixture::new() else { return };
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    db.admin.batch_execute("ALTER TABLE case_subject_revisions RENAME CONSTRAINT subject_projection TO unrelated_check").unwrap();
    assert!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err());
}
