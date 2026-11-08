use crate::case_administration_support;
use application::cases::*;
use application::ApplicationError;
use case_administration_support::Fixture;
use domain::cases::{CaseId, CaseMetadata};
use infrastructure::PostgresAuditLog;

#[test]
fn startup_rejects_missing_history_noncanonical_provenance_and_lost_initial_stage() {
    for sql in [
        "DELETE FROM case_administration_revisions WHERE revision=1",
        "DELETE FROM case_initial_stage_registrations",
    ] {
        let Some(mut f) = Fixture::new() else { return };
        let store = f.store();
        let id = CaseId::new();
        let profile = PenalCaseProfile::new(
            "NUC",
            "Office",
            "Folder",
            "Court",
            &["Reported"],
            None,
            None,
        )
        .unwrap();
        store
            .register_penal(
                f.owner,
                id,
                PenalCaseCreation::new(CaseMetadata::new("New", "REF").unwrap(), profile),
                f.at,
            )
            .unwrap();
        for revision in [1, 2] {
            store
                .change_administrative_status(
                    f.owner,
                    id,
                    CaseRevisionExpectation::new(revision),
                    CaseAdministrativeStatus::Active,
                    f.at,
                )
                .unwrap();
        }
        f.admin
            .batch_execute("SET session_replication_role=replica")
            .unwrap();
        f.admin.batch_execute(sql).unwrap();
        f.admin
            .batch_execute("SET session_replication_role=origin")
            .unwrap();
        assert!(
            matches!(
                PostgresAuditLog::open(&f.runtime_url),
                Err(ApplicationError::InvalidConfiguration(_))
            ),
            "accepted {sql}"
        );
    }
}
