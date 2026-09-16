mod case_administration_support;

use case_administration_support::Fixture;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;

#[test]
fn runtime_requires_read_insert_only_on_typed_history_and_pure_helper_execution() {
    let Some(db) = Fixture::new() else { return };
    let mut runtime = db.runtime();
    for table in [
        "case_subjects",
        "case_subject_revisions",
        "case_participant_typed_revisions",
        "subject_identity_reviews",
        "participant_identity_reviews",
        "participant_credential_evidence",
    ] {
        let row=runtime.query_one("SELECT has_table_privilege(current_user,$1,'SELECT'),has_table_privilege(current_user,$1,'INSERT')",&[&table]).unwrap();
        assert!(row.get::<_, bool>(0) && row.get::<_, bool>(1), "{table}");
        for sql in [
            format!("DELETE FROM {table}"),
            format!("TRUNCATE {table}"),
            format!("ALTER TABLE {table} DISABLE TRIGGER ALL"),
        ] {
            assert_eq!(
                runtime
                    .batch_execute(&sql)
                    .unwrap_err()
                    .code()
                    .unwrap()
                    .code(),
                "42501"
            );
        }
    }
    assert!(runtime
        .query_one("SELECT typed_u32(decode('00000001','hex'),0)", &[])
        .is_ok());
    assert_eq!(
        runtime
            .batch_execute("SELECT preserve_typed_history()")
            .unwrap_err()
            .code()
            .unwrap()
            .code(),
        "42501"
    );
}

#[test]
fn runtime_startup_rejects_typed_write_escalation_and_trigger_function_ownership() {
    for grant in [
        "GRANT UPDATE ON case_subjects TO ROLE",
        "GRANT UPDATE(display_name) ON case_subject_revisions TO ROLE",
        "GRANT DELETE ON participant_identity_reviews TO ROLE",
        "GRANT TRUNCATE ON participant_credential_evidence TO ROLE",
        "GRANT EXECUTE ON FUNCTION typed_credential_guard() TO ROLE",
        "ALTER FUNCTION typed_u32(BYTEA,INTEGER) OWNER TO ROLE",
        "GRANT UPDATE ON case_participant_typed_revisions TO PUBLIC",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
        db.admin
            .batch_execute(&grant.replace("ROLE", &db.role))
            .unwrap();
        assert!(
            PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
            "{grant}"
        );
    }
}
