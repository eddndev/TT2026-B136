#[path = "case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod precautionary_hearing_backend_support;

use precautionary_hearing_backend_support::*;

#[test]
fn postgres_schedule_returns_the_exact_confirmed_capture_and_original_prefix() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let workflow = service(&db, actor.clone());
    let draft = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    let stored = workflow
        .submit("session", db.case, command.clone(), confirmation(&draft))
        .unwrap();

    assert_eq!(stored.capture.review, draft);
    assert_eq!(stored.capture.review.actor, actor);
    assert_eq!(stored.capture.review.command, command);
    assert_eq!(stored.capture.review.result_revision.get(), 1);
    assert_eq!(stored.capture.recorded_at, db.at);
    assert_eq!(stored.capture.recorded_at.nanosecond(), 123_456_789);
    assert_eq!(stored.history.captures, vec![stored.capture.clone()]);
    assert!(stored.history.measure_history.groups.is_empty());
}
