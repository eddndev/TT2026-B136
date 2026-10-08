use super::*;
use application::typed_participants::SubjectSnapshot;
use domain::precautionary_hearings::MeasureId;

#[path = "replacement_support.rs"]
mod support;
use support::*;
#[path = "replacement_atomic.rs"]
mod atomic;
#[path = "replacement_consumers.rs"]
mod consumers;
#[path = "replacement_corruption.rs"]
mod corruption;
#[path = "replacement_sql.rs"]
mod sql;

#[test]
fn replacement_atomically_marks_the_old_identity_and_creates_a_linked_administrative_root() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, subject, command) = setup_joint(&mut db);
    let previous = &judicial.group.measures[0];
    let storage = store(&db);
    let MeasureAdministrativePreparation::Ready(ready) = storage
        .prepare(
            &seed.actor,
            db.case,
            &command,
            &application::documents::StageSupportReadLimits::standard(),
        )
        .unwrap()
    else {
        panic!("expected a fresh replacement")
    };
    assert_eq!(ready.replacement_subject, Some(subject.clone()));
    let stored = persist(&db, seed.actor.clone(), command.clone());
    assert_retained(&stored, &judicial, previous);
    assert_eq!(stored.capture.review.result.values, previous.result.values);
    assert_joint(&stored, &subject, &previous.result.values);
    let replacement = stored.capture.review.replacement.as_ref().unwrap();
    assert_eq!(replacement.judicial_origin, previous.result.origin);
    assert_eq!(replacement.last_judicial.reference, reference(previous));
    assert_eq!(replacement.last_action, previous.result.action);
    assert_eq!(
        replacement.sources.supervisor,
        previous.result.sources.supervisor
    );
    assert_eq!(stored.capture.records[0].result.id, replacement.id);
    assert_eq!(stored.capture.review.result.id, previous.result.id);
    assert_eq!(stored.record_history.records.judicial.groups.len(), 1);
    assert_eq!(
        stored.record_history.records.judicial.groups[0].capture,
        judicial.group
    );
    assert_heads(&db, &seed.actor, &stored);
    let counts = db
        .admin
        .query_one(
            "SELECT (SELECT count(*) FROM case_measure_operations WHERE family='a1'),
          (SELECT count(*) FROM case_measure_revisions WHERE owner_operation=$1),
          (SELECT count(*) FROM case_measures WHERE root_operation=$1),
          (SELECT count(*) FROM audit_events WHERE action='measure_administrative.recorded')",
            &[&command.operation_id.as_uuid()],
        )
        .unwrap();
    assert_eq!(
        (
            counts.get::<_, i64>(0),
            counts.get::<_, i64>(1),
            counts.get::<_, i64>(2),
            counts.get::<_, i64>(3)
        ),
        (1, 2, 1, 1)
    );
    db.migrate();
    reopened(&db, &seed.actor, &stored);
    assert_heads(&db, &seed.actor, &stored);
}

#[test]
fn replacement_from_a_corrected_record_retains_effective_terms_and_original_judicial_evidence() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, subject, mut command) = setup_joint(&mut db);
    let first = persist(
        &db,
        seed.actor.clone(),
        correction(
            command.target,
            command.context,
            &judicial.group.measures[0].result.values,
            "The effective transcription before identity replacement",
        ),
    );
    command.target = corrected_reference(&first.capture);
    let stored = persist(&db, seed.actor.clone(), command);
    assert_retained(&stored, &judicial, &judicial.group.measures[0]);
    assert_eq!(stored.capture.review.result.revision.get(), 3);
    assert_eq!(
        stored.capture.review.result.values,
        first.capture.review.result.values
    );
    assert_joint(&stored, &subject, &first.capture.review.result.values);
    assert_eq!(stored.record_history.records.administrative.len(), 1);
    assert_eq!(
        stored.record_history.records.administrative[0].capture,
        first.capture
    );
    let new_row = replacement_row(&stored);
    let later = persist(
        &db,
        seed.actor.clone(),
        correction(
            row_reference(new_row),
            seed.command.context,
            &new_row.result.values,
            "Correct the new identity's transcription",
        ),
    );
    assert_eq!(
        later.capture.review.result.record_root,
        new_row.result.record_root
    );
    assert_eq!(
        later.capture.review.result.judicial_origin,
        new_row.result.judicial_origin
    );
    assert_eq!(
        later.capture.review.result.last_judicial,
        new_row.result.last_judicial
    );
    assert_eq!(later.capture.review.result.revision.get(), 2);
    reopened(&db, &seed.actor, &stored);
    reopened(&db, &seed.actor, &later);
    reopened(&db, &seed.actor, &first);
}
