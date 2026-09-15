#[allow(dead_code)]
mod participant_database_support;

use infrastructure::PostgresCaseDocumentStore;
use participant_database_support::{configuration_error, Fixture};

#[test]
fn runtime_requires_participant_tables_constraints_and_active_triggers() {
    for corruption in [
        "DROP TABLE case_participant_revisions CASCADE",
        "ALTER TABLE case_participants DROP CONSTRAINT participant_case_fk",
        "ALTER TABLE case_participants DROP CONSTRAINT participant_first_revision_fk",
        "ALTER TABLE case_participants DROP CONSTRAINT participant_initial_revision",
        "ALTER TABLE case_participant_revisions DROP CONSTRAINT participant_first_active",
        "ALTER TABLE case_participant_revisions DROP CONSTRAINT participant_canonical",
        "ALTER TABLE case_participant_revisions DROP CONSTRAINT participant_digest",
        "ALTER TABLE case_participant_revisions DROP CONSTRAINT participant_actor_fk",
        "ALTER TABLE case_participant_revisions DISABLE TRIGGER participant_sequence",
        "ALTER TABLE case_participants DISABLE TRIGGER participant_root_immutable",
        "ALTER TABLE case_participant_revisions DISABLE TRIGGER participant_revision_immutable",
        "ALTER TABLE case_participants ALTER CONSTRAINT participant_first_revision_fk NOT DEFERRABLE",
    ] {
        let Some(mut f) = Fixture::new() else { return };
        PostgresCaseDocumentStore::open(&f.runtime_url).unwrap();
        f.admin.batch_execute(corruption).unwrap();
        assert!(configuration_error(PostgresCaseDocumentStore::open(&f.runtime_url)), "accepted {corruption}");
    }
}

#[test]
fn runtime_rejects_participant_history_rewrite_or_ownership_privileges() {
    let Some(mut f) = Fixture::new() else { return };
    for table in ["case_participants", "case_participant_revisions"] {
        for privilege in ["UPDATE", "DELETE", "TRUNCATE", "TRIGGER"] {
            f.admin
                .batch_execute(&format!("GRANT {privilege} ON {table} TO {}", f.role))
                .unwrap();
            let opened = PostgresCaseDocumentStore::open(&f.runtime_url);
            f.admin
                .batch_execute(&format!("REVOKE {privilege} ON {table} FROM {}", f.role))
                .unwrap();
            assert!(configuration_error(opened), "accepted {table} {privilege}");
        }
    }
}

#[test]
fn startup_detects_missing_initial_or_intermediate_history_and_invalid_provenance() {
    use application::participants::{ParticipantId, ParticipantRevision, ParticipantStore};
    for corruption in [
        "DELETE FROM case_participant_revisions WHERE revision=1",
        "DELETE FROM case_participant_revisions WHERE revision=2",
        "UPDATE case_participant_revisions SET changed_at='2025-01-01T01:00:00+01:00'",
        "UPDATE case_participant_revisions SET changed_by_email=''",
        "UPDATE case_participant_revisions SET changed_by='00000000-0000-0000-0000-000000000001'",
    ] {
        let Some(mut f) = Fixture::new() else { return };
        let store = f.store();
        let id = ParticipantId::new();
        store
            .create(
                f.owner,
                f.case,
                id,
                participant_database_support::values("Original"),
                f.at,
            )
            .unwrap();
        for revision in 1..=2 {
            store
                .replace(
                    f.owner,
                    f.case,
                    id,
                    ParticipantRevision::new(revision).unwrap(),
                    participant_database_support::values("Later"),
                    f.at,
                )
                .unwrap();
        }
        f.admin
            .batch_execute("SET session_replication_role=replica")
            .unwrap();
        f.admin.batch_execute(corruption).unwrap();
        f.admin
            .batch_execute("SET session_replication_role=origin")
            .unwrap();
        assert!(
            configuration_error(PostgresCaseDocumentStore::open(&f.runtime_url)),
            "accepted {corruption}"
        );
    }
}

#[test]
fn function_ownership_column_update_and_unvalidated_constraints_are_rejected() {
    for alteration in [
        "ALTER FUNCTION participant_values_bytes(TEXT,TEXT,TEXT,TEXT,TEXT) OWNER TO ROLE",
        "ALTER FUNCTION participant_values_is_canonical(TEXT,TEXT,TEXT,TEXT,TEXT) OWNER TO ROLE",
        "ALTER FUNCTION enforce_participant_sequence() OWNER TO ROLE",
        "ALTER FUNCTION preserve_participant_history() OWNER TO ROLE",
        "GRANT UPDATE(changed_by_email) ON case_participant_revisions TO ROLE",
        "ALTER TABLE case_participant_revisions DROP CONSTRAINT participant_digest; ALTER TABLE case_participant_revisions ADD CONSTRAINT participant_digest CHECK(values_digest=pg_catalog.sha256(participant_values_bytes(display_name,procedural_role,organization,legal_status,directory_status))) NOT VALID",
        "ALTER TABLE case_participant_revisions ALTER COLUMN changed_by_email DROP NOT NULL",
    ] {
        let Some(mut f)=Fixture::new() else {return};
        f.admin.batch_execute(&alteration.replace("ROLE",&f.role)).unwrap();
        assert!(configuration_error(PostgresCaseDocumentStore::open(&f.runtime_url)),"accepted {alteration}");
    }
}
