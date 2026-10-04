use super::*;
use application::precautionary_measures::{
    measure_group_origin, MeasureDecisionGroupCapture, MeasureGroupEvidence, MeasureHistoryEvidence,
};
use domain::precautionary_measures::{
    MeasureDecisionId, MeasureDecisionOperationId, MeasureDecisionOutcome,
    MeasureDecisionOutcomeInput, MeasureEffect,
};

fn list_operations(
    items: Vec<PrecautionaryHearingStoredOperation>,
) -> Result<PrecautionaryHearingPage, ApplicationError> {
    let case_id = items[0].capture.review.case_id;
    let actor = items[0].capture.review.actor.clone();
    let returned = page(case_id, items);
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(returned));
    let mut identity = crate::case_support::MockIdentity::new();
    identity
        .expect_authenticate()
        .times(1..=2)
        .returning(move |_| Ok(actor.clone()));
    service(store, identity, clock()).list(
        "session",
        case_id,
        PrecautionaryHearingReadQuery::default(),
    )
}

fn review_operation(
    group: &MeasureDecisionGroupCapture,
    hearing: u128,
) -> PrecautionaryHearingStoredOperation {
    let empty = MeasureHistoryEvidence { groups: vec![] };
    let history = MeasureHistoryEvidence {
        groups: vec![MeasureGroupEvidence {
            origin: measure_group_origin(&Hasher, group, &empty).unwrap(),
            capture: group.clone(),
        }],
    };
    let mut fixture = crate::receipt_support::Fixture::schedule();
    let mut input = crate::receipt_support::values_input();
    input.purpose = PrecautionaryHearingPurpose::Review;
    input.review_targets = vec![crate::measure_decision_fixtures::reference(
        &group.measures[0],
    )];
    fixture.command.hearing_id = id(hearing);
    fixture.command.operation_id =
        PrecautionaryHearingOperationId::from_uuid(uuid::Uuid::from_u128(1000 + hearing));
    fixture.command.change = PrecautionaryHearingChange::Schedule {
        context: crate::receipt_support::expectation(&fixture.context),
        values: PrecautionaryHearingValues::new(input).unwrap(),
    };
    let capture = prepare_precautionary_hearing_with_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command,
        PrecautionaryHearingPreparationMaterial {
            observed_context: fixture.context,
            sources: fixture.sources,
            predecessor: None,
            measure_history: &history,
        },
    )
    .unwrap()
    .into_capture(&Hasher, group.recorded_at + time::Duration::seconds(1))
    .unwrap();
    let origin =
        precautionary_hearing_origin_with_measure_history(&Hasher, &capture, &history).unwrap();
    PrecautionaryHearingStoredOperation {
        history: PrecautionaryHearingHistoryEvidence {
            origin,
            captures: vec![capture.clone()],
            measure_history: history,
        },
        capture,
    }
}

#[test]
fn regression_page_rejects_one_hearing_operation_claimed_by_two_hearing_ids() {
    let first = operation(20);
    let mut fixture = crate::receipt_support::Fixture::schedule();
    fixture.command.hearing_id = id(30);
    fixture.command.operation_id = first.capture.review.command.operation_id;
    let second = stored(fixture.capture(None, at()));
    assert!(list_operations(vec![first, second]).is_err());
}

#[test]
fn regression_page_rejects_one_decision_id_claimed_by_distinct_group_operations() {
    let first = crate::measure_decision_fixtures::Fixture::single().capture();
    let mut fixture = crate::measure_decision_fixtures::Fixture::single();
    fixture.command.operation_id =
        MeasureDecisionOperationId::from_uuid(uuid::Uuid::from_u128(101));
    let mut effects = fixture.command.outcome.changes().unwrap().to_vec();
    let MeasureEffect::Impose(proposal) = &mut effects[0] else {
        unreachable!()
    };
    proposal.id = MeasureId::from_uuid(uuid::Uuid::from_u128(80));
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
    fixture.material.result_sources[0].id = MeasureId::from_uuid(uuid::Uuid::from_u128(80));
    let second = fixture.capture();
    assert_eq!(
        first.review.command.decision_id,
        second.review.command.decision_id
    );
    assert_ne!(first.measures[0].result.id, second.measures[0].result.id);
    assert!(list_operations(vec![
        review_operation(&first, 20),
        review_operation(&second, 30)
    ])
    .is_err());
}

#[test]
fn regression_page_rejects_one_measure_revision_owned_by_distinct_decision_groups() {
    let first = crate::measure_decision_fixtures::Fixture::single().capture();
    let mut fixture = crate::measure_decision_fixtures::Fixture::single();
    fixture.command.operation_id =
        MeasureDecisionOperationId::from_uuid(uuid::Uuid::from_u128(101));
    fixture.command.decision_id = MeasureDecisionId::from_uuid(uuid::Uuid::from_u128(111));
    let second = fixture.capture();
    assert_eq!(first.measures[0].result.id, second.measures[0].result.id);
    assert_eq!(
        first.measures[0].result.revision,
        second.measures[0].result.revision
    );
    assert_ne!(
        first.review.command.decision_id,
        second.review.command.decision_id
    );
    assert!(list_operations(vec![
        review_operation(&first, 20),
        review_operation(&second, 30)
    ])
    .is_err());
}

#[test]
fn distinct_hearings_may_share_the_same_exact_measure_group_evidence() {
    let group = crate::measure_decision_fixtures::Fixture::single().capture();
    let expected = vec![review_operation(&group, 20), review_operation(&group, 30)];
    assert_eq!(list_operations(expected.clone()).unwrap().items, expected);
}
