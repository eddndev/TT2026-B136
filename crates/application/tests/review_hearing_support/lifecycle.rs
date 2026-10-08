use super::*;
use time::Duration;

#[test]
fn replacement_and_cancellation_validate_the_union_of_old_and_new_measure_revisions() {
    let first_group = MeasureFixture::single().capture();
    let first_history = history(&first_group);
    let first = ReviewFixture::schedule(&first_group).capture(None, first_group.recorded_at);
    let origin =
        precautionary_hearing_origin_with_measure_history(&Hasher, &first, &first_history).unwrap();
    let second_group = MeasureChangeFixture::confirm(&first_group).capture();
    let union =
        crate::measure_decision_effect_support::append_history(&first_history, &second_group);
    let second = ReviewFixture::replace(&first, targets(&second_group), union.clone()).capture(
        Some(&first),
        second_group.recorded_at + Duration::seconds(1),
    );
    assert_eq!(
        second.review.result_revision,
        PrecautionaryHearingRevision::new(2).unwrap()
    );
    assert_eq!(
        second.review.resolved_values.review_targets(),
        targets(&second_group)
    );
    precautionary_hearing_transition_with_measure_history_matches(&Hasher, &first, &second, &union)
        .unwrap();
    let cancelled = ReviewFixture::cancel(&second, union.clone())
        .capture(Some(&second), second.recorded_at + Duration::seconds(1));
    assert_eq!(
        cancelled.review.resolved_values,
        second.review.resolved_values
    );
    assert_eq!(cancelled.review.sources, second.review.sources);
    assert_eq!(
        cancelled.review.scheduling_context,
        second.review.scheduling_context
    );
    assert_eq!(
        cancelled.review.status,
        domain::hearings::HearingStatus::Cancelled
    );
    precautionary_hearing_transition_with_measure_history_matches(
        &Hasher, &second, &cancelled, &union,
    )
    .unwrap();
    precautionary_hearing_history_with_measure_history_matches(
        &Hasher,
        &[first.clone(), second.clone(), cancelled.clone()],
        &origin,
        &union,
    )
    .unwrap();
    assert!(precautionary_hearing_origin_with_measure_history(&Hasher, &second, &union).is_err());
    assert!(
        precautionary_hearing_origin_with_measure_history(&Hasher, &cancelled, &union).is_err()
    );
    assert!(precautionary_hearing_transition_matches(&Hasher, &first, &second).is_err());
    assert!(
        precautionary_hearing_history_matches(&Hasher, &[first, second, cancelled], &origin)
            .is_err()
    );
}

#[test]
fn a_history_union_can_resolve_sixty_four_refs_for_thirty_two_measure_identities() {
    let mut initial = MeasureFixture::single();
    let sources = initial.material.result_sources[0].sources.clone();
    let values = domain::precautionary_measures::MeasureValues::new(
        crate::measure_source_support::input(&sources),
    );
    for id in 71..102 {
        initial.add_imposition(id, values.clone(), sources.clone());
    }
    let first_group = initial.capture();
    let first_history = history(&first_group);
    let mut update = MeasureChangeFixture::confirm(&first_group);
    update.effects(
        first_group
            .measures
            .iter()
            .map(
                |measure| domain::precautionary_measures::MeasureEffect::Confirm {
                    previous: crate::measure_decision_fixtures::reference(measure),
                },
            )
            .collect(),
    );
    update.request.material.predecessors = first_group
        .measures
        .iter()
        .map(|member| crate::measure_decision_effect_support::owned_member(&first_group, member))
        .collect();
    update.request.material.result_sources = first_group
        .measures
        .iter()
        .map(|member| MeasureResultSources {
            id: member.result.id,
            sources: member.result.sources.clone(),
        })
        .collect();
    let second_group = update.capture();
    let union =
        crate::measure_decision_effect_support::append_history(&first_history, &second_group);
    let first = ReviewFixture::schedule(&first_group).capture(None, first_group.recorded_at);
    let origin =
        precautionary_hearing_origin_with_measure_history(&Hasher, &first, &first_history).unwrap();
    let second_fixture = ReviewFixture::replace(&first, targets(&second_group), union.clone());
    let mut reversed = second_fixture.clone();
    reversed.measure_history.groups.reverse();
    let at = second_group.recorded_at + Duration::seconds(1);
    let second = second_fixture.capture(Some(&first), at);
    assert_eq!(reversed.capture(Some(&first), at), second);
    assert_eq!(first.review.resolved_values.review_targets().len(), 32);
    assert_eq!(second.review.resolved_values.review_targets().len(), 32);
    precautionary_hearing_transition_with_measure_history_matches(&Hasher, &first, &second, &union)
        .unwrap();
    precautionary_hearing_history_with_measure_history_matches(
        &Hasher,
        &[first, second],
        &origin,
        &union,
    )
    .unwrap();
}

#[test]
fn replacement_cannot_omit_old_target_ownership_just_because_the_new_values_change_targets() {
    let first_group = MeasureFixture::single().capture();
    let first = ReviewFixture::schedule(&first_group).capture(None, first_group.recorded_at);
    let mut unrelated = MeasureFixture::single();
    unrelated.command.operation_id =
        domain::precautionary_measures::MeasureDecisionOperationId::from_uuid(
            uuid::Uuid::from_u128(200),
        );
    unrelated.command.decision_id =
        domain::precautionary_measures::MeasureDecisionId::from_uuid(uuid::Uuid::from_u128(210));
    let sources = unrelated.material.result_sources[0].sources.clone();
    unrelated.command.outcome = domain::precautionary_measures::MeasureDecisionOutcome::new(
        domain::precautionary_measures::MeasureDecisionOutcomeInput::Changes(vec![
            domain::precautionary_measures::MeasureEffect::Impose(
                domain::precautionary_measures::MeasureProposal {
                    id: crate::measure_decision_fixtures::id(80),
                    values: domain::precautionary_measures::MeasureValues::new(
                        crate::measure_source_support::input(&sources),
                    ),
                },
            ),
        ]),
    )
    .unwrap();
    unrelated.material.result_sources[0].id = crate::measure_decision_fixtures::id(80);
    let new_group = unrelated.capture();
    let incomplete = history(&new_group);
    let fixture = ReviewFixture::replace(&first, targets(&new_group), incomplete.clone());
    assert!(fixture.prepare(Some(&first)).is_err());
    let mut complete = history(&first_group);
    complete.groups.extend(incomplete.groups);
    let second = ReviewFixture::replace(&first, targets(&new_group), complete.clone())
        .capture(Some(&first), first.recorded_at + Duration::seconds(2));
    precautionary_hearing_transition_with_measure_history_matches(
        &Hasher, &first, &second, &complete,
    )
    .unwrap();
}

#[test]
fn cancellation_is_terminal_and_retains_exact_target_values_and_previous_digest() {
    let group = MeasureFixture::single().capture();
    let evidence = history(&group);
    let first = ReviewFixture::schedule(&group).capture(None, group.recorded_at);
    let cancelled = ReviewFixture::cancel(&first, evidence.clone())
        .capture(Some(&first), first.recorded_at + Duration::seconds(2));
    assert!(
        ReviewFixture::replace(&cancelled, targets(&group), evidence.clone())
            .prepare(Some(&cancelled))
            .is_err()
    );
    assert!(ReviewFixture::cancel(&cancelled, evidence.clone())
        .prepare(Some(&cancelled))
        .is_err());
    let mut wrong = ReviewFixture::cancel(&first, evidence);
    let PrecautionaryHearingChange::Cancel {
        expected_capture_digest,
        ..
    } = &mut wrong.hearing.command.change
    else {
        unreachable!()
    };
    *expected_capture_digest = domain::crypto::Sha256Digest::from_array([99; 32]);
    assert!(wrong.prepare(Some(&first)).is_err());
}
