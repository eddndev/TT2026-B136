mod case_administration_support;

use application::cases::*;
use application::ApplicationError;
use case_administration_support::Fixture;
use domain::cases::{CaseId, CaseMetadata};
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;

fn repository(f: &Fixture) -> PostgresCaseRepository {
    PostgresCaseRepository::open(&f.runtime_url, Arc::new(RingSha256Hasher)).unwrap()
}
fn creation(nuc: &str) -> PenalCaseCreation {
    PenalCaseCreation::new(
        CaseMetadata::new("Original", "REF").unwrap(),
        PenalCaseProfile::new(
            nuc,
            "Authority",
            nuc,
            "Court",
            &["Reported offense"],
            Some("Line1\nLine2"),
            None,
        )
        .unwrap(),
    )
}
#[test]
fn new_basic_and_penal_cases_commit_initial_revisions_and_exact_stage_provenance() {
    let Some(mut f) = Fixture::new() else { return };
    let store = repository(&f);
    let id = CaseId::new();
    let record = store
        .create_basic(
            f.owner,
            id,
            CaseMetadata::new("Basic", "REF").unwrap(),
            f.at,
        )
        .unwrap();
    assert_eq!(record.id, id);
    let initial = store.get_administration(f.owner, id, f.at).unwrap();
    assert_eq!(initial.administration.revision(), Some(CaseRevision::FIRST));
    assert!(initial.administration.values().profile().is_none());
    assert!(initial.initial_stage.is_none());
    let penal = CaseId::new();
    let first = store
        .register_penal(f.owner, penal, creation("NUC"), f.at)
        .unwrap();
    let stage = first.initial_stage.clone().unwrap();
    assert_eq!(stage.recorded_at, f.at);
    assert_eq!(stage.recorded_by.email, "owner@example.test");
    assert_eq!(
        stage.administration_digest,
        first.administration.snapshot().unwrap().values_digest
    );
    f.admin
        .execute(
            "UPDATE users SET email='renamed@example.test' WHERE id=$1",
            &[&f.owner.as_uuid()],
        )
        .unwrap();
    let second = store
        .replace_administration(
            f.owner,
            penal,
            CaseRevisionExpectation::new(1),
            CaseEditableValues::new(
                CaseMetadata::new("Edited", "NewRef").unwrap(),
                first.administration.values().profile().cloned(),
            ),
            f.at,
        )
        .unwrap();
    assert_eq!(second.initial_stage, Some(stage));
    assert_eq!(
        second.administration.snapshot().unwrap().changed_by.email,
        "renamed@example.test"
    );
    assert_eq!(
        store.get_basic(f.owner, penal, f.at).unwrap().title,
        "Edited"
    );
    let actions: Vec<String>=f.admin.query("SELECT action FROM audit_events WHERE action IN ('case.created','case.administration_created','case.penal_registered','case.stage_initialized') ORDER BY sequence", &[]).unwrap().iter().map(|r|r.get(0)).collect();
    assert_eq!(
        actions,
        vec![
            "case.created",
            "case.administration_created",
            "case.created",
            "case.penal_registered",
            "case.stage_initialized"
        ]
    );
}
#[test]
fn baseline_closure_and_completion_preserve_absence_of_initial_stage_and_enforce_cas() {
    let Some(f) = Fixture::new() else { return };
    let store = repository(&f);
    let baseline = store.get_administration(f.owner, f.case, f.at).unwrap();
    assert!(matches!(
        baseline.administration,
        CurrentCaseAdministration::Unrevised(_)
    ));
    assert!(store
        .administration_history(
            f.owner,
            f.case,
            CaseAdministrationHistoryQuery::new(10, None).unwrap(),
            f.at
        )
        .unwrap()
        .revisions
        .is_empty());
    let closed = store
        .change_administrative_status(
            f.owner,
            f.case,
            CaseRevisionExpectation::new(0),
            CaseAdministrativeStatus::Closed,
            f.at,
        )
        .unwrap();
    assert_eq!(
        closed.administration.values().metadata().title(),
        "Baseline"
    );
    assert!(matches!(
        store.replace_administration(
            f.owner,
            f.case,
            CaseRevisionExpectation::new(1),
            CaseEditableValues::new(CaseMetadata::new("Edit", "REF").unwrap(), None),
            f.at
        ),
        Err(ApplicationError::CaseClosed)
    ));
    assert!(matches!(
        store.change_administrative_status(
            f.owner,
            f.case,
            CaseRevisionExpectation::new(0),
            CaseAdministrativeStatus::Active,
            f.at
        ),
        Err(ApplicationError::CaseRevisionConflict)
    ));
    store
        .change_administrative_status(
            f.owner,
            f.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Active,
            f.at,
        )
        .unwrap();
    let ready = store
        .replace_administration(
            f.owner,
            f.case,
            CaseRevisionExpectation::new(2),
            creation("BaselineNUC").into_values().editable().clone(),
            f.at,
        )
        .unwrap();
    assert!(ready.initial_stage.is_none());
    assert!(matches!(
        store.replace_administration(
            f.owner,
            f.case,
            CaseRevisionExpectation::new(3),
            CaseEditableValues::new(CaseMetadata::new("Edit", "REF").unwrap(), None),
            f.at
        ),
        Err(ApplicationError::CaseProfileRequired)
    ));
}
#[test]
fn failed_audit_rolls_back_complete_creation_and_history_reads_return_no_data() {
    let Some(mut f) = Fixture::new() else { return };
    let store = repository(&f);
    f.admin.batch_execute("ALTER TABLE audit_events ADD CONSTRAINT reject_new_case_stage CHECK(action<>'case.stage_initialized')").unwrap();
    let id = CaseId::new();
    assert!(matches!(
        store.register_penal(f.owner, id, creation("Rollback"), f.at),
        Err(ApplicationError::Port(_))
    ));
    let count: i64 = f
        .admin
        .query_one("SELECT count(*) FROM cases WHERE id=$1", &[&id.as_uuid()])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
    let count: i64 = f
        .admin
        .query_one("SELECT count(*) FROM audit_events", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
    f.admin.batch_execute("ALTER TABLE audit_events DROP CONSTRAINT reject_new_case_stage; ALTER TABLE audit_events ADD CONSTRAINT reject_history_read CHECK(action<>'case.administration_history_read')").unwrap();
    assert!(matches!(
        store.administration_history(
            f.owner,
            f.case,
            CaseAdministrationHistoryQuery::new(10, None).unwrap(),
            f.at
        ),
        Err(ApplicationError::Port(_))
    ));
}
