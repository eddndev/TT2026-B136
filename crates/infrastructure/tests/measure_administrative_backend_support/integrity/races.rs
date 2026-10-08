use super::*;
use std::sync::Mutex;

#[test]
fn a_review_committed_during_admission_blocks_the_administrative_write_atomically() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let draft = service(&db, seed.actor.clone())
        .prepare("session", db.case, command.clone())
        .unwrap();
    let hearing_command = hearing_command(&db, &seed, vec![command.target]);
    let hearing_workflow = crate::hearing_fixture::service(&db, seed.actor.clone());
    let hearing_review = hearing_workflow
        .prepare("session", db.case, hearing_command.clone())
        .unwrap();
    let case = db.case;
    let admin_url = db.admin_url.clone();
    let after_dependant = Arc::new(Mutex::new(None));
    let captured = after_dependant.clone();
    let workflow = service_with_format(
        &db,
        seed.actor,
        FormatCheck(Some(Box::new(move || {
            hearing_workflow
                .submit(
                    "session",
                    case,
                    hearing_command.clone(),
                    crate::hearing_fixture::confirmation(&hearing_review),
                )
                .unwrap();
            let mut client = postgres::Client::connect(&admin_url, postgres::NoTls).unwrap();
            *captured.lock().unwrap() = Some(inventory_snapshot(&mut client));
        }))),
    );

    let result = workflow.submit("session", db.case, command, confirmation(&draft));

    assert!(matches!(
        result,
        Err(ApplicationError::MeasureAdministrative(
            MeasureAdministrativeError::KnownDependants
        ))
    ));
    let expected = after_dependant
        .lock()
        .unwrap()
        .clone()
        .expect("admission callback ran");
    assert_eq!(inventory_snapshot(&mut db.admin), expected);
}

#[test]
fn an_unrelated_sibling_review_during_admission_preserves_the_reviewed_correction() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, command) = setup(&mut db);
    let draft = service(&db, seed.actor.clone())
        .prepare("session", db.case, command.clone())
        .unwrap();
    let sibling = reference(&judicial.group.measures[1]);
    let hearing_command = hearing_command(&db, &seed, vec![sibling]);
    let hearing_workflow = crate::hearing_fixture::service(&db, seed.actor.clone());
    let hearing_review = hearing_workflow
        .prepare("session", db.case, hearing_command.clone())
        .unwrap();
    let case = db.case;
    let hearing_result = Arc::new(Mutex::new(None));
    let captured = hearing_result.clone();
    let workflow = service_with_format(
        &db,
        seed.actor.clone(),
        FormatCheck(Some(Box::new(move || {
            let stored = hearing_workflow
                .submit(
                    "session",
                    case,
                    hearing_command.clone(),
                    crate::hearing_fixture::confirmation(&hearing_review),
                )
                .unwrap();
            *captured.lock().unwrap() = Some(stored);
        }))),
    );

    let stored = workflow
        .submit("session", db.case, command, confirmation(&draft))
        .unwrap();

    assert_eq!(stored.capture.review, draft);
    let hearing = hearing_result
        .lock()
        .unwrap()
        .clone()
        .expect("admission callback ran");
    assert_eq!(
        hearing.capture.review.resolved_values.review_targets(),
        &[sibling]
    );
    assert_eq!(stored.record_history.records.judicial.groups.len(), 1);
    reopened(&db, &seed.actor, &stored);
}
