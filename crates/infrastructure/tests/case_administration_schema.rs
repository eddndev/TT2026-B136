use crate::case_administration_support;
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
        "ALTER TABLE case_administration_revisions DROP CONSTRAINT case_administration_digest",
        "ALTER TABLE cases DISABLE TRIGGER case_root_immutable",
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
        "GRANT INSERT(created_at) ON cases TO ROLE",
        "GRANT UPDATE(title) ON cases TO ROLE",
        "ALTER TABLE case_initial_stage_registrations OWNER TO ROLE",
    ] {
        let Some(mut f) = Fixture::new() else { return };
        f.admin
            .batch_execute(&sql.replace("ROLE", &f.role))
            .unwrap();
        assert!(rejected(&f.runtime_url), "accepted {sql}");
    }
}

#[test]
fn startup_rejects_invalid_baseline_creation_time_without_changing_rows() {
    let Some(mut f) = Fixture::new() else { return };
    f.admin.batch_execute("ALTER TABLE cases DISABLE TRIGGER case_root_immutable; ALTER TABLE cases DROP CONSTRAINT case_created_at_finite; UPDATE cases SET created_at='infinity'; ALTER TABLE cases ADD CONSTRAINT case_created_at_finite CHECK(isfinite(created_at)) NOT VALID; ALTER TABLE cases ENABLE TRIGGER case_root_immutable").unwrap();
    assert!(rejected(&f.runtime_url));
}
