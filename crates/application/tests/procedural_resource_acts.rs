#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod procedural_fact_service_support;
mod procedural_resource_support;
use application::{procedural_resources::*, ApplicationError};
use domain::identity::Role;
use procedural_resource_support::*;
use std::sync::Arc;

#[test]
fn act_and_its_correction_preserve_resource_sources_and_do_not_readmit_retained_support() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    let base = commit(
        &actor,
        fixture.case_id,
        fixture.command.clone(),
        fixture.material.clone(),
    );
    let old_capture = base.receipt.capture_digest;
    let mut material = retained(base.clone());
    material.records = fixture.material.records.clone();
    let command = ResourceCommand {
        operation_id: ResourceOperationId::new(),
        resource_id: base.id,
        change: ResourceChange::RecordAct {
            expected_revision: base.revision,
            act_id: ResourceActId::new(),
            values: act_values(
                fixture.values.resolution_evidence().clone(),
                "Declared filing",
            ),
        },
    };
    let act = commit(&actor, fixture.case_id, command, material);
    assert_eq!(act.sources, base.sources);
    assert_eq!(act.receipt.previous.unwrap().capture_digest, old_capture);
    assert_eq!(
        act.act.as_ref().unwrap().revision,
        ResourceActRevision::initial()
    );
    let mut material = retained(act.clone());
    material.act_base = Some(act.clone());
    let prior = act.act.as_ref().unwrap();
    let command = ResourceCommand {
        operation_id: ResourceOperationId::new(),
        resource_id: base.id,
        change: ResourceChange::CorrectAct {
            expected_revision: act.revision,
            act_id: prior.id,
            expected_act_revision: prior.revision,
            reason: text("Correct declaration"),
            values: act_values(
                fixture.values.resolution_evidence().clone(),
                "Corrected filing declaration",
            ),
        },
    };
    let (identity, _) = identity_for(actor, 2);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(ResourcePreparation::Ready(Box::new(material))));
    let validator = Arc::new(Validator::default());
    let service = service(store, identity, validator.clone());
    let corrected = service
        .prepare("session", fixture.case_id, command)
        .unwrap();
    assert_eq!(validator.calls(), 0);
    assert_eq!(corrected.sources, base.sources);
    let changed = corrected.act.unwrap();
    assert_eq!(changed.revision.get(), 2);
    assert_eq!(changed.supports, prior.supports);
    assert_eq!(
        changed.previous.unwrap().capture_digest,
        act.receipt.capture_digest
    );
    assert_eq!(prior.values.statement().as_str(), "Declared filing");
}

#[test]
fn archive_and_reactivation_change_organizational_status_without_act_or_new_admission() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    let base = commit(&actor, fixture.case_id, fixture.command, fixture.material);
    let command = ResourceCommand {
        operation_id: ResourceOperationId::new(),
        resource_id: base.id,
        change: ResourceChange::Archive {
            expected_revision: base.revision,
            reason: text("Organizational archive"),
        },
    };
    let archived = commit(&actor, fixture.case_id, command, retained(base.clone()));
    assert_eq!(archived.status, ResourceStatus::Archived);
    assert_eq!(archived.values, base.values);
    assert_eq!(archived.sources, base.sources);
    assert!(archived.act.is_none());
    let command = ResourceCommand {
        operation_id: ResourceOperationId::new(),
        resource_id: base.id,
        change: ResourceChange::Reactivate {
            expected_revision: archived.revision,
            reason: text("Organizational reactivation"),
        },
    };
    let active = commit(&actor, fixture.case_id, command, retained(archived));
    assert_eq!(active.status, ResourceStatus::Active);
    assert_eq!(active.values, base.values);
    assert_eq!(active.sources, base.sources);
    assert!(active.act.is_none());
}

#[test]
fn closed_case_and_stale_resource_revision_fail_before_new_admission() {
    for closed in [true, false] {
        let (identity, actor) = case_support::identity(Role::Owner, 1);
        let fixture = Fixture::new(&actor);
        let base = commit(
            &actor,
            fixture.case_id,
            fixture.command.clone(),
            fixture.material,
        );
        let mut material = retained(base.clone());
        if closed {
            use application::cases::*;
            let values = material
                .administration
                .values()
                .with_status(CaseAdministrativeStatus::Closed);
            material.administration =
                CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
                    case_id: fixture.case_id,
                    revision: CaseRevision::new(1).unwrap(),
                    values_digest: case_administration_digest(
                        procedural_fact_service_support::hasher().as_ref(),
                        &values,
                    ),
                    values,
                    changed_at: case_support::instant(),
                    changed_by: base.recorded_by.clone(),
                }));
        }
        let command = ResourceCommand {
            operation_id: ResourceOperationId::new(),
            resource_id: base.id,
            change: ResourceChange::Archive {
                expected_revision: ResourceRevision::new(if closed { 1 } else { 2 }).unwrap(),
                reason: text("Archive"),
            },
        };
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .return_once(move |_, _, _, _| Ok(ResourcePreparation::Ready(Box::new(material))));
        let validator = Arc::new(Validator::default());
        let service = service(store, identity, validator.clone());
        let result = service.prepare("session", fixture.case_id, command);
        if closed {
            assert!(matches!(result, Err(ApplicationError::CaseClosed)));
        } else {
            assert!(matches!(
                result,
                Err(ApplicationError::ProceduralResource(
                    ProceduralResourceError::RevisionConflict
                ))
            ));
        }
        assert_eq!(validator.calls(), 0);
    }
}

#[test]
fn capture_commitment_rejects_changed_author_appellant_support_and_timestamp() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    let base = commit(&actor, fixture.case_id, fixture.command, fixture.material);
    for field in 0..4 {
        let mut changed = base.clone();
        match field {
            0 => changed.recorded_by.email = "substituted@example.test".into(),
            1 => changed.sources.appellants[0].overview.display_name = "Substituted person".into(),
            2 => changed.sources.supports[0].name = "substituted.pdf".into(),
            _ => changed.recorded_at += time::Duration::seconds(1),
        }
        assert!(resource_receipt_matches(
            procedural_fact_service_support::hasher().as_ref(),
            &changed
        )
        .is_err());
    }
}
