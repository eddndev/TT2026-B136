use super::*;
use domain::hearings::{HearingNote, HearingStatus, HearingVenue};

fn replacement(
    prior: &PrecautionaryHearingRecordStoredOperation,
    target: PrecautionaryMeasureRef,
) -> PrecautionaryHearingCommand {
    let capture = &prior.capture;
    let mut values = crate::hearing_fixture::input(&capture.review.resolved_values);
    values.review_targets = vec![target];
    values.venue = HearingVenue::new("Updated court for the corrected record").unwrap();
    PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: capture.review.command.hearing_id,
        change: PrecautionaryHearingChange::Replace {
            expected_revision: capture.review.result_revision,
            expected_capture_digest: capture.capture_digest,
            context: crate::hearing_fixture::expectation(&capture.review.observed_context),
            values: PrecautionaryHearingValues::new(values).unwrap(),
            reason: HearingNote::new("Declared new review target and venue").unwrap(),
        },
    }
}

fn cancellation(prior: &PrecautionaryHearingRecordStoredOperation) -> PrecautionaryHearingCommand {
    PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: prior.capture.review.command.hearing_id,
        change: PrecautionaryHearingChange::Cancel {
            expected_revision: prior.capture.review.result_revision,
            expected_capture_digest: prior.capture.capture_digest,
            reason: HearingNote::new("Declared appointment cancellation").unwrap(),
        },
    }
}

#[test]
fn replacing_between_exact_corrected_revisions_and_cancelling_retains_the_original_prefix() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let next = crate::administrative_fixture::persist(
        &db,
        seed.actor.clone(),
        crate::administrative_fixture::correction(
            crate::administrative_fixture::corrected_reference(&seed.corrected.capture),
            seed.corrected.capture.review.command.context,
            &seed.corrected.capture.review.result.values,
            "Second corrected conditions before any hearing dependency",
        ),
    );
    let first = persist(&db, seed.actor.clone(), seed.command);
    let replaced = persist(
        &db,
        seed.actor.clone(),
        replacement(
            &first,
            crate::administrative_fixture::corrected_reference(&next.capture),
        ),
    );
    let cancelled = persist(&db, seed.actor.clone(), cancellation(&replaced));
    assert_eq!(cancelled.capture.review.status, HearingStatus::Cancelled);
    assert_eq!(
        cancelled.capture.review.resolved_values,
        replaced.capture.review.resolved_values
    );
    assert_eq!(
        cancelled.capture.review.sources,
        replaced.capture.review.sources
    );
    assert_eq!(
        cancelled.capture.review.scheduling_context,
        replaced.capture.review.scheduling_context
    );
    assert_eq!(cancelled.history.origin, first.history.origin);
    assert_eq!(
        cancelled.history.captures,
        vec![
            first.capture.clone(),
            replaced.capture.clone(),
            cancelled.capture.clone()
        ]
    );
    assert_eq!(
        cancelled
            .history
            .record_history
            .records
            .judicial
            .groups
            .len(),
        1
    );
    assert_eq!(
        cancelled.history.record_history.records.judicial.groups[0]
            .capture
            .measures
            .len(),
        2
    );
    assert_eq!(
        cancelled
            .history
            .record_history
            .records
            .administrative
            .len(),
        2
    );
    for original in [&first, &replaced, &cancelled] {
        reopened(&db, &seed.actor, original);
    }
    assert_eq!(first.history.record_history.records.administrative.len(), 1);
}

#[test]
fn older_valid_correction_remains_selectable_after_a_later_mark_without_using_the_marked_head() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let marked = crate::administrative_fixture::persist(
        &db,
        seed.actor.clone(),
        crate::administrative_fixture::mark(
            crate::administrative_fixture::corrected_reference(&seed.corrected.capture),
            seed.corrected.capture.review.command.context,
        ),
    );
    assert_eq!(
        marked.capture.review.result.validity,
        application::measure_corrections::MeasureCaptureValidity::EnteredInError
    );
    let original = persist(&db, seed.actor.clone(), seed.command);
    let cancelled = persist(&db, seed.actor.clone(), cancellation(&original));
    for result in [&original, &cancelled] {
        assert_eq!(
            result.history.record_history.records.administrative.len(),
            1
        );
        assert_eq!(
            result.history.record_history.records.administrative[0].capture,
            seed.corrected.capture
        );
        reopened(&db, &seed.actor, result);
    }
}

#[test]
fn a_legacy_hearing_replays_through_the_mixed_workflow_without_changing_its_capture() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let mut command = seed.command;
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut command.change else {
        unreachable!()
    };
    let mut input = crate::hearing_fixture::input(values);
    input.purpose = PrecautionaryHearingPurpose::Imposition;
    input.review_targets.clear();
    *values = PrecautionaryHearingValues::new(input).unwrap();
    let legacy = crate::hearing_fixture::persist(&db, seed.actor.clone(), command.clone());
    let workflow = service_with_format(
        &db,
        seed.actor,
        FormatCheck(Some(Box::new(|| {
            panic!("legacy replay through mixed workflow must skip admission")
        }))),
    );
    let replayed = workflow
        .submit(
            "session",
            db.case,
            command,
            confirmation(&legacy.capture.review),
        )
        .unwrap();
    assert_eq!(replayed.capture, legacy.capture);
    assert_eq!(replayed.history.origin, legacy.history.origin);
    assert_eq!(replayed.history.captures, legacy.history.captures);
    assert!(replayed
        .history
        .record_history
        .records
        .judicial
        .groups
        .is_empty());
    assert!(replayed
        .history
        .record_history
        .records
        .administrative
        .is_empty());
    assert!(replayed.history.record_history.decisions.is_empty());
}
