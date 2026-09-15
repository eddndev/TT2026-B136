mod case_administration_support;

use application::ApplicationError;
use case_administration_support::Fixture;
use infrastructure::PostgresAuditLog;

fn rejected(url: &str) -> bool {
    matches!(
        PostgresAuditLog::open(url),
        Err(ApplicationError::InvalidConfiguration(_))
    )
}

#[test]
fn runtime_requires_administration_structure_defaults_and_deferred_guards() {
    for sql in [
        "DROP TABLE case_initial_stage_registrations CASCADE",
        "ALTER TABLE cases DROP CONSTRAINT case_first_administration_revision_fk",
        "ALTER TABLE cases ALTER COLUMN required_initial_revision DROP DEFAULT",
        "ALTER TABLE cases ALTER CONSTRAINT case_first_administration_revision_fk NOT DEFERRABLE",
        "ALTER TABLE cases DROP CONSTRAINT case_root_metadata_canonical",
        "ALTER TABLE case_administration_revisions DROP CONSTRAINT case_administration_digest",
        "ALTER TABLE case_administration_revisions DISABLE TRIGGER case_administration_heads_valid",
        "ALTER TABLE case_administration_revisions DISABLE TRIGGER case_administration_sequence",
        "ALTER TABLE case_initial_stage_registrations DISABLE TRIGGER case_initial_stage_valid",
        "ALTER TABLE cases DISABLE TRIGGER case_root_immutable",
        "ALTER TABLE case_administration_revisions ALTER COLUMN changed_by_email DROP NOT NULL",
        "ALTER FUNCTION case_administration_bytes(TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT[],TEXT,TEXT) VOLATILE",
        "ALTER TABLE cases DROP CONSTRAINT case_created_at_finite; ALTER TABLE cases ADD CONSTRAINT case_created_at_finite CHECK(isfinite(created_at)) NOT VALID",
    ] {
        let Some(mut f) = Fixture::new() else { return };
        PostgresAuditLog::open(&f.runtime_url).unwrap();
        f.admin.batch_execute(sql).unwrap();
        assert!(rejected(&f.runtime_url), "accepted {sql}");
    }
}

#[test]
fn runtime_cannot_own_or_rewrite_case_administration_or_insert_baselines() {
    for sql in [
        "GRANT INSERT ON cases TO ROLE",
        "GRANT INSERT(required_initial_revision) ON cases TO ROLE",
        "GRANT INSERT(created_at) ON cases TO ROLE",
        "GRANT UPDATE(title) ON cases TO ROLE",
        "GRANT DELETE ON cases TO ROLE",
        "GRANT TRUNCATE ON case_administration_revisions TO ROLE",
        "GRANT UPDATE ON case_initial_stage_registrations TO ROLE",
        "GRANT TRIGGER ON case_administration_revisions TO ROLE",
        "ALTER FUNCTION case_administration_bytes(TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT[],TEXT,TEXT) OWNER TO ROLE",
        "ALTER FUNCTION validate_case_administration_heads() OWNER TO ROLE",
        "ALTER FUNCTION enforce_case_administration_sequence() OWNER TO ROLE",
        "ALTER TABLE case_initial_stage_registrations OWNER TO ROLE",
    ] {
        let Some(mut f) = Fixture::new() else { return };
        f.admin.batch_execute(&sql.replace("ROLE", &f.role)).unwrap();
        assert!(rejected(&f.runtime_url), "accepted {sql}");
    }
}

#[test]
fn startup_rejects_invalid_baseline_creation_time_without_changing_rows() {
    let Some(mut f) = Fixture::new() else { return };
    f.admin.batch_execute("ALTER TABLE cases DISABLE TRIGGER case_root_immutable; ALTER TABLE cases DROP CONSTRAINT case_created_at_finite; UPDATE cases SET created_at='infinity'; ALTER TABLE cases ADD CONSTRAINT case_created_at_finite CHECK(isfinite(created_at)) NOT VALID; ALTER TABLE cases ENABLE TRIGGER case_root_immutable").unwrap();
    assert!(rejected(&f.runtime_url));
}
