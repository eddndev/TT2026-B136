use super::*;
use application::cases::CaseRevisionExpectation;
use domain::crypto::DocumentHasher;
use postgres::error::SqlState;

fn rehash(group: &mut MeasureDecisionGroupCapture) {
    let review = &mut group.review;
    review.submission_digest = RingSha256Hasher.hash_bytes(
        &measure_decision_submission_bytes(&review.actor, review.case_id, &review.command).unwrap(),
    );
    review.review_digest =
        RingSha256Hasher.hash_bytes(&measure_decision_review_bytes(review).unwrap());
    group.decision.capture_digest =
        RingSha256Hasher.hash_bytes(&measure_decision_capture_bytes(&group.decision).unwrap());
    assert!(group.measures.is_empty());
    group.capture_digest =
        RingSha256Hasher.hash_bytes(&measure_decision_group_bytes(group).unwrap());
}

fn reject(db: &mut Fixture, group: &MeasureDecisionGroupCapture, reason: &str) {
    let before = snapshot(db);
    let error = direct::insert(db, group, &direct::row(group)).expect_err(reason);
    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    assert_eq!(snapshot(db), before);
}

#[test]
fn direct_sql_decision_cannot_predate_its_exact_precautionary_capture() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let original_at = db.at;
    db.at += Duration::seconds(10);
    seed.hearing = crate::hearing_fixture::persist(
        &db,
        seed.seed.actor.clone(),
        hearing_command(&db, &seed.seed, vec![]),
    );
    seed.command.anchor = Some(anchor(&seed.hearing));
    let mut candidate = group(&db, &seed.seed.actor, &seed.command);
    direct::insert(&db, &candidate, &direct::row(&candidate))
        .expect("a decision at the exact hearing capture time is valid");
    candidate.recorded_at = original_at + Duration::seconds(9);
    candidate.decision.recorded_at = candidate.recorded_at;
    rehash(&mut candidate);
    assert!(measure_decision_group_matches(&RingSha256Hasher, &candidate).is_err());

    reject(
        &mut db,
        &candidate,
        "accepted a decision before its exact hearing capture",
    );
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
            creation(&format!(
                "ANCHOR-CONTEXT-{}-{}",
                db.case,
                previous.material().administration.revision.get()
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

#[test]
fn direct_sql_decision_context_cannot_regress_its_selected_hearings_observed_clock() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let original_at = db.at;
    let second = replace_administration(
        &db,
        &seed.hearing.capture.review.observed_context,
        original_at + Duration::seconds(10),
    );
    db.at = original_at + Duration::seconds(10);
    let mut appointment = hearing_command(&db, &seed.seed, vec![]);
    let PrecautionaryHearingChange::Schedule { context, .. } = &mut appointment.change else {
        unreachable!()
    };
    *context = crate::hearing_fixture::expectation(&second);
    seed.hearing = crate::hearing_fixture::persist(&db, seed.seed.actor.clone(), appointment);
    seed.command.context = crate::hearing_fixture::expectation(&second);
    seed.command.anchor = Some(anchor(&seed.hearing));
    db.at = original_at + Duration::seconds(11);
    let mut candidate = group(&db, &seed.seed.actor, &seed.command);
    direct::insert(&db, &candidate, &direct::row(&candidate))
        .expect("the exact nonregressing observed context is valid");

    let third = replace_administration(&db, &second, original_at + Duration::seconds(9));
    assert!(
        third.material().administration.changed_at < second.material().administration.changed_at
    );
    candidate.review.command.context = crate::hearing_fixture::expectation(&third);
    candidate.review.material.context = third.clone();
    candidate.decision.context = third;
    rehash(&mut candidate);
    assert!(measure_decision_group_matches(&RingSha256Hasher, &candidate).is_err());

    reject(
        &mut db,
        &candidate,
        "accepted a later administration revision with an earlier source clock",
    );
}

#[test]
fn direct_sql_anchor_rechecks_historical_participant_source_chronology() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let mut appointment = hearing_command(&db, &seed.seed, vec![]);
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut appointment.change else {
        unreachable!()
    };
    let mut input = crate::hearing_fixture::input(values);
    input.participants = vec![domain::hearings::HearingParticipantRef::new(
        seed.seed.supervisor.id(),
        seed.seed.supervisor.revision_number(),
    )];
    *values = PrecautionaryHearingValues::new(input).unwrap();
    seed.hearing = crate::hearing_fixture::persist(&db, seed.seed.actor.clone(), appointment);
    seed.command.anchor = Some(anchor(&seed.hearing));
    db.at += Duration::seconds(10);
    let candidate = group(&db, &seed.seed.actor, &seed.command);
    direct::insert(&db, &candidate, &direct::row(&candidate))
        .expect("the exact participant source is valid");

    let future = (db.at + Duration::seconds(10))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute("ALTER TABLE case_participant_typed_revisions DISABLE TRIGGER ALL")
        .unwrap();
    assert_eq!(tx.execute(
        "UPDATE case_participant_typed_revisions SET changed_at=$1 WHERE participant_id=$2 AND revision=$3",
        &[&future, &seed.seed.supervisor.id().as_uuid(), &i64::from(seed.seed.supervisor.revision_number().get())],
    ).unwrap(), 1);
    tx.batch_execute("ALTER TABLE case_participant_typed_revisions ENABLE TRIGGER ALL")
        .unwrap();
    tx.commit().unwrap();

    reject(
        &mut db,
        &candidate,
        "accepted an anchor captured before its historical participant source",
    );
}
