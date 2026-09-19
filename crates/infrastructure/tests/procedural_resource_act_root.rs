mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;
mod procedural_resource_support;
use application::{procedural_resources::*, ApplicationError};
use domain::identity::Role;
use procedural_resource_support::*;

#[test]
fn exact_corrected_act_rejects_root_redirected_to_another_act_after_open() {
    reject_redirected_root(false);
}

#[test]
fn corrected_act_history_rejects_root_redirected_to_its_correction_after_open() {
    reject_redirected_root(true);
}

fn reject_redirected_root(history: bool) {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let resource = persist(&workflow, db.case, registration(&db));
    let first_act = persist(&workflow, db.case, act(&resource));
    let other_act = persist(&workflow, db.case, act(&first_act));
    let original = first_act.act.as_ref().unwrap();
    let corrected = persist(
        &workflow,
        db.case,
        ResourceCommand {
            operation_id: ResourceOperationId::new(),
            resource_id: resource.id,
            change: ResourceChange::CorrectAct {
                expected_revision: other_act.revision,
                act_id: original.id,
                expected_act_revision: original.revision,
                values: original.values.clone(),
                reason: text("Correct declared filing"),
            },
        },
    );
    let adapter = store(&db);
    assert_eq!(
        adapter
            .get(
                db.owner,
                db.case,
                resource.id,
                Some(corrected.revision),
                db.at
            )
            .unwrap(),
        corrected
    );
    assert_eq!(
        adapter
            .history(
                db.owner,
                db.case,
                resource.id,
                ResourceHistoryQuery::new(1, None).unwrap(),
                db.at
            )
            .unwrap()
            .revisions,
        vec![corrected.clone()]
    );
    let redirected = if history {
        corrected.revision
    } else {
        other_act.revision
    };
    // Both targets retain the same case/resource foreign key and existing revision.
    db.admin
        .batch_execute("ALTER TABLE case_procedural_resource_acts DISABLE TRIGGER USER")
        .unwrap();
    db.admin
        .execute(
            "UPDATE case_procedural_resource_acts SET initial_resource_revision=$2 WHERE id=$1",
            &[&original.id.as_uuid(), &i64::from(redirected.get())],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE case_procedural_resource_acts ENABLE TRIGGER USER")
        .unwrap();
    let before = snapshot(&mut db);
    let result = if history {
        adapter
            .history(
                db.owner,
                db.case,
                resource.id,
                ResourceHistoryQuery::new(1, None).unwrap(),
                db.at,
            )
            .map(|_| ())
    } else {
        adapter
            .get(
                db.owner,
                db.case,
                resource.id,
                Some(corrected.revision),
                db.at,
            )
            .map(|_| ())
    };
    assert!(matches!(
        result,
        Err(ApplicationError::ProceduralResource(
            ProceduralResourceError::StoredInconsistent(_)
        ))
    ));
    assert_eq!(snapshot(&mut db), before);
}
