mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod procedural_fact_backend_support;
#[path = "deadline_profile_interleaved_support/rendezvous.rs"]
mod rendezvous;
use application::{deadline_profiles::DeadlineProfileCollection, deadlines::*, ApplicationError};
use deadline_backend_support::*;
use domain::identity::Role;
use std::{sync::Arc, time::Duration};

#[test]
fn simultaneous_connections_commit_one_successor_and_one_audit_event() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist(&workflow, db.case, setup(&db));
    let left = prepared(&db, db.owner, &correct(&first));
    let right = prepared(&db, db.owner, &correct(&first));
    let before = snapshot(&mut db);
    let gate = Arc::new(rendezvous::PrepareRendezvous::new(Duration::from_secs(5)));
    let mut threads = Vec::new();
    for pending in [left, right] {
        let adapter = store(&db);
        let actor = db.owner;
        let gate = gate.clone();
        threads.push(std::thread::spawn(move || {
            gate.wait();
            adapter.commit(actor, pending)
        }));
    }
    let joined: Vec<_> = threads.into_iter().map(|thread| thread.join()).collect();
    let results: Vec<_> = joined.into_iter().map(Result::unwrap).collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(ApplicationError::Deadline(DeadlineError::RevisionConflict))
            ))
            .count(),
        1
    );
    let after = snapshot(&mut db);
    assert_eq!(after["roots"], before["roots"]);
    assert_eq!(
        after["revisions"].as_array().unwrap().len(),
        before["revisions"].as_array().unwrap().len() + 1
    );
    assert_eq!(
        after["audit"].as_array().unwrap().len(),
        before["audit"].as_array().unwrap().len() + 1
    );
    assert_eq!(after["events"], before["events"]);
    assert_eq!(
        workflow
            .get("session", db.case, first.id, None)
            .unwrap()
            .revision
            .get(),
        2
    );
}

#[test]
fn attention_and_retirement_keep_sources_retired_after_registration_and_a_revoked_responsible() {
    let Some(mut db) = Fixture::new() else { return };
    let responsible = db.user("paralegal", true);
    let profile = profile(&db);
    let source = source(&db);
    let mut command = command(&db, &profile, &source);
    definition_mut(&mut command).responsible = responsible;
    let workflow = service(&db, db.owner, Role::Owner);
    let original = persist(&workflow, db.case, command);
    let profiles = crate::deadline_profile_database_support::service(&db, db.owner, Role::Owner);
    crate::deadline_profile_database_support::persist(
        &profiles,
        DeadlineProfileCollection::ForCase(db.case),
        crate::deadline_profile_database_support::retire(&profile),
    );
    let facts = crate::procedural_fact_backend_support::service(&db, db.owner, Role::Owner);
    crate::procedural_fact_backend_support::persist(
        &facts,
        db.case,
        crate::procedural_fact_backend_support::withdraw(&source),
    );
    db.admin
        .execute(
            "UPDATE users SET active=false,email='former-responsible@example.test' WHERE id=$1",
            &[&responsible.as_uuid()],
        )
        .unwrap();
    let attended = persist(&workflow, db.case, attention(&original));
    let retired = persist(&workflow, db.case, retire(&attended));
    for detail in [&attended, &retired] {
        assert_eq!(detail.definition, original.definition);
        assert_eq!(detail.calculation, original.calculation);
        assert_eq!(detail.responsible, original.responsible);
        assert_eq!(
            workflow
                .get("session", db.case, detail.id, Some(detail.revision))
                .unwrap(),
            *detail
        );
    }
    assert_eq!(retired.attention, attended.attention);
    assert_eq!(
        workflow
            .get("session", db.case, original.id, Some(original.revision))
            .unwrap(),
        original
    );
}
