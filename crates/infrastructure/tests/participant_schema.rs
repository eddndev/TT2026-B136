use crate::participant_database_support;
use infrastructure::PostgresCaseDocumentStore;
use participant_database_support::{configuration_error, Fixture};

#[test]
fn runtime_requires_participant_tables_constraints_and_active_triggers() {
    for corruption in [
        "ALTER TABLE case_participant_revisions DROP CONSTRAINT participant_digest",
        "ALTER TABLE case_participant_revisions DISABLE TRIGGER participant_revision_immutable",
    ] {
        let Some(mut f) = Fixture::new() else { return };
        PostgresCaseDocumentStore::open(&f.runtime_url).unwrap();
        f.admin.batch_execute(corruption).unwrap();
        assert!(
            configuration_error(PostgresCaseDocumentStore::open(&f.runtime_url)),
            "accepted {corruption}"
        );
    }
}

#[test]
fn runtime_rejects_participant_history_rewrite_or_ownership_privileges() {
    let Some(mut f) = Fixture::new() else { return };
    f.admin
        .batch_execute(&format!(
            "GRANT UPDATE ON case_participant_revisions TO {}",
            f.role
        ))
        .unwrap();
    assert!(
        configuration_error(PostgresCaseDocumentStore::open(&f.runtime_url)),
        "accepted case_participant_revisions UPDATE"
    );
}

#[test]
fn startup_detects_missing_initial_or_intermediate_history_and_invalid_provenance() {
    use application::participants::{ParticipantId, ParticipantRevision, ParticipantStore};
    for corruption in [
        "DELETE FROM case_participant_revisions WHERE revision=1",
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
        "ALTER FUNCTION enforce_participant_sequence() OWNER TO ROLE",
        "GRANT UPDATE(changed_by_email) ON case_participant_revisions TO ROLE",
        "ALTER TABLE case_participant_revisions DROP CONSTRAINT participant_digest; ALTER TABLE case_participant_revisions ADD CONSTRAINT participant_digest CHECK(values_digest=pg_catalog.sha256(participant_values_bytes(display_name,procedural_role,organization,legal_status,directory_status))) NOT VALID",
    ] {
        let Some(mut f)=Fixture::new() else {return};
        f.admin.batch_execute(&alteration.replace("ROLE",&f.role)).unwrap();
        assert!(configuration_error(PostgresCaseDocumentStore::open(&f.runtime_url)),"accepted {alteration}");
    }
}
