use super::*;
use application::cases::{CaseRepository, CaseRevisionExpectation};
use domain::crypto::DocumentHasher;
use postgres::error::SqlState;

mod append;

#[test]
fn direct_sql_review_replacement_rejects_a_regressed_observed_administration_clock() {
    check_administration_clock(false);
}

#[test]
fn direct_sql_review_cancellation_rejects_a_regressed_observed_administration_clock() {
    check_administration_clock(true);
}

fn check_administration_clock(cancel: bool) {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let original_at = db.at;
    let initial_context = &seed.first.group.review.material.context;
    let second = replace_administration(&db, initial_context, original_at + Duration::seconds(10));
    if let PrecautionaryHearingChange::Schedule { context, .. } = &mut seed.command.change {
        *context = crate::hearing_fixture::expectation(&second);
    } else {
        unreachable!();
    }
    db.at = original_at + Duration::seconds(10);
    let first = persist_hearing(&db, seed.actor.clone(), seed.command);
    db.at = original_at + Duration::seconds(11);
    let command = if cancel {
        cancellation(&first)
    } else {
        crate::hearing_fixture::replacement(&first)
    };
    let mut candidate = prepare_precautionary_hearing_with_history(
        &RingSha256Hasher,
        &seed.actor,
        db.case,
        command,
        PrecautionaryHearingPreparationMaterial {
            observed_context: second.clone(),
            sources: first.capture.review.sources.clone(),
            predecessor: Some(&first.capture),
            measure_history: &first.history.measure_history,
        },
    )
    .unwrap()
    .into_capture(&RingSha256Hasher, db.at)
    .unwrap();
    append::insert(&db, &candidate)
        .expect("the direct append accepts an unchanged valid observed context");

    let third = replace_administration(&db, &second, original_at + Duration::seconds(9));
    assert_eq!(third.material().administration.revision.get(), 3);
    assert!(
        third.material().administration.changed_at < second.material().administration.changed_at
    );
    candidate.review.observed_context = third.clone();
    if let PrecautionaryHearingChange::Replace { context, .. } =
        &mut candidate.review.command.change
    {
        *context = crate::hearing_fixture::expectation(&third);
        candidate.review.scheduling_context = third.clone();
    }
    let review = &mut candidate.review;
    review.submission_digest = RingSha256Hasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &review.actor,
            review.case_id,
            &review.command,
            &review.resolved_values,
        )
        .unwrap(),
    );
    review.review_digest =
        RingSha256Hasher.hash_bytes(&precautionary_hearing_review_bytes(review).unwrap());
    candidate.capture_digest =
        RingSha256Hasher.hash_bytes(&precautionary_hearing_capture_bytes(&candidate).unwrap());
    assert!(
        prepare_precautionary_hearing_with_history(
            &RingSha256Hasher,
            &seed.actor,
            db.case,
            candidate.review.command.clone(),
            PrecautionaryHearingPreparationMaterial {
                observed_context: third,
                sources: first.capture.review.sources.clone(),
                predecessor: Some(&first.capture),
                measure_history: &first.history.measure_history,
            },
        )
        .is_err(),
        "the application rejects the same regressed source clock"
    );

    let before = snapshot(&mut db);
    let error = append::insert(&db, &candidate)
        .expect_err("accepted a newer hearing with an earlier observed administration clock");
    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    assert_eq!(snapshot(&mut db), before);
}

fn replace_administration(
    db: &Fixture,
    previous: &PrecautionaryContext,
    at: time::OffsetDateTime,
) -> PrecautionaryContext {
    let detail = db
        .store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(previous.material().administration.revision),
            crate::hearing_fixture::creation(&format!(
                "CONTEXT-{}-{}",
                db.case,
                previous.material().administration.revision.get(),
            ))
            .into_values()
            .editable()
            .clone(),
            at,
        )
        .unwrap();
    let mut material = previous.material().clone();
    material.administration = detail.administration.snapshot().unwrap().clone();
    PrecautionaryContext::new(&RingSha256Hasher, material).unwrap()
}
