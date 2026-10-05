use super::*;

fn list(
    items: Vec<MeasureAdministrativeStoredOperation>,
) -> Result<MeasureAdministrativePage, ApplicationError> {
    list_result(
        MeasureAdministrativeReadQuery::default(),
        page(crate::participant_support::case_id(), items),
    )
}

#[test]
fn exact_shared_owners_may_be_both_page_items_and_later_items_ancestors() {
    let first = operation(10);
    let second = super::mixed::next(&first, 20);
    let third = super::mixed::next(&second, 30);
    let expected = vec![first, second, third];
    assert_eq!(list(expected.clone()).unwrap().items, expected);
}

#[test]
fn page_rejects_distinct_judicial_operation_decision_and_member_owners() {
    let first = operation(10);
    let prior = &first.record_history.records.judicial.groups[0];
    for mutation in 0..4 {
        let mut fixture = root_fixture(20);
        match mutation {
            0 => fixture.command.operation_id = prior.origin.operation_id,
            1 => fixture.command.decision_id = prior.origin.decision_id,
            2 => {
                let measure = prior.capture.measures[0].result.id;
                let mut effects = fixture.command.outcome.changes().unwrap().to_vec();
                let MeasureEffect::Impose(proposal) = &mut effects[0] else {
                    unreachable!()
                };
                proposal.id = measure;
                fixture.command.outcome =
                    MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects))
                        .unwrap();
                fixture.material.result_sources[0].id = measure;
            }
            _ => {
                fixture.command.operation_id =
                    MeasureDecisionOperationId::from_uuid(first.origin.operation_id.as_uuid())
            }
        }
        let second = stored(&from_group(
            &fixture.capture(),
            &crate::effect_support::empty_history(),
            20,
        ));
        assert!(list(vec![first.clone(), second]).is_err());
    }
}

#[test]
fn page_rejects_disagreement_on_a_repeated_administrative_ancestor() {
    let first = operation(10);
    let mut other = from_group(
        &root_fixture(20).capture(),
        &crate::effect_support::empty_history(),
        10,
    );
    other.command.reason = note("Another individually valid declaration");
    let second = stored(&other);
    let first = super::mixed::next(&first, 100);
    let second = super::mixed::next(&second, 200);
    assert!(list(vec![first, second]).is_err());
}

#[test]
fn page_rejects_a_revision_owned_by_an_administrative_item_and_a_judicial_ancestor() {
    let first = operation(10);
    let previous = &first.record_history.records.judicial.groups[0].capture;
    let next = crate::effect_support::LaterFixture::next(
        previous,
        &crate::effect_support::empty_history(),
        99,
    );
    let history = next.evidence.clone();
    let second = stored(&from_group(&next.capture(), &history, 20));
    assert_eq!(
        first.capture.records[0].result.revision,
        second.record_history.records.judicial.groups[1]
            .capture
            .measures[0]
            .result
            .revision
    );
    assert!(list(vec![first, second]).is_err());
}

#[test]
fn individually_valid_items_cannot_disagree_on_shared_full_sources() {
    let first = operation(10);
    for mutation in 0..4 {
        let mut fixture = root_fixture(20);
        let sources = &mut fixture.material.result_sources[0].sources;
        match mutation {
            0 => {
                sources.subject.changed_by.email = "different-subject-recorder@example.test".into()
            }
            1 => {
                sources
                    .supervisor
                    .as_mut()
                    .unwrap()
                    .bound_subject
                    .as_mut()
                    .unwrap()
                    .changed_by
                    .email = "different-bound-subject@example.test".into()
            }
            2 => {
                crate::participant_support::typed_mut(sources.supervisor.as_mut().unwrap())
                    .changed_by
                    .email = "different-participant@example.test".into()
            }
            _ => fixture.material.support.name = "different-name.pdf".into(),
        }
        let second = stored(&from_group(
            &fixture.capture(),
            &crate::effect_support::empty_history(),
            20,
        ));
        assert!(list(vec![first.clone(), second]).is_err());
    }
}

#[test]
fn repeated_g2_owner_is_compared_across_individually_valid_mixed_receipts() {
    let first = super::mixed::after_m2();
    let g2 = &first.record_history.decisions[0];
    let mut fixture = FixtureV2::initial(root_fixture(99));
    fixture.command.operation_id = g2.origin.operation_id;
    fixture.command.decision_id = g2.origin.decision_id;
    let replacement = fixture.capture();
    let row = &replacement.measures[0];
    let mut command = first.capture.review.command.clone();
    command.operation_id = operation_id(30);
    command.target = reference_v2(row);
    let second = capture(
        command,
        replacement.review.material.context.clone(),
        append_v2(&fixture.history, &replacement),
        replacement.recorded_at + Duration::seconds(1),
    );
    assert!(list(vec![first, second]).is_err());
}
