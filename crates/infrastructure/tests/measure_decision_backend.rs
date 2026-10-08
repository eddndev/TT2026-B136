#[path = "case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod measure_decision_backend_support;
#[allow(unused_imports)]
#[path = "typed_participant_service_support/mod.rs"]
mod typed_participant_service_support;

use measure_decision_backend_support::*;

#[test]
fn postgres_imposition_returns_the_exact_confirmed_group_and_actual_subject_sources() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let workflow = service(&db, seed.actor.clone());
    let review = workflow
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    let stored = workflow
        .submit(
            "session",
            db.case,
            seed.command.clone(),
            confirmation(&review),
        )
        .unwrap();
    assert_eq!(stored.group.review, review);
    assert_eq!(stored.group.review.actor, seed.actor);
    assert_eq!(stored.group.review.command, seed.command);
    assert_eq!(stored.group.recorded_at, db.at);
    assert_eq!(stored.group.recorded_at.nanosecond(), 123_456_789);
    assert_eq!(stored.group.measures.len(), 2);
    assert!(stored.measure_history.groups.is_empty());
    for measure in &stored.group.measures {
        assert_eq!(measure.result.sources.subject, seed.subject);
        assert!(measure.result.sources.supervisor.is_none());
        assert_eq!(measure.result.revision.get(), 1);
        assert_eq!(measure.recorded_at, db.at);
        assert_eq!(
            measure.decision_digest,
            stored.group.decision.capture_digest
        );
    }
}
