use super::*;

fn list(items: Vec<MeasureRecordDetail>) -> Result<MeasureRecordPage, ApplicationError> {
    list_result(
        MeasureRecordReadQuery::default(),
        page(crate::participant_support::case_id(), items),
    )
}
fn member(
    group: &MeasureDecisionGroupCapture,
    history: &MeasureHistoryEvidence,
    index: usize,
) -> MeasureRecordDetail {
    let mut result = from_group(group, history);
    result.reference = reference(&group.measures[index]);
    result.record = OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V1(Box::new(
        crate::effect_support::owned_member(group, &group.measures[index]),
    )));
    result
}

#[test]
fn different_current_heads_can_share_the_exact_same_owner_and_ancestor_sources() {
    let group = crate::measure_decision_fixtures::Fixture::multiple().capture();
    let history = crate::effect_support::empty_history();
    let first = administrative(&member(&group, &history, 0), 10, false);
    let second = member(&group, &history, 1);
    let expected = vec![first, second];
    assert_eq!(list(expected.clone()).unwrap().items, expected);
}

#[test]
fn distinct_page_roots_cannot_reuse_judicial_operation_or_decision_identity() {
    let first = initial(10);
    let origin = &first.record_history.records.judicial.groups[0].origin;
    for operation in [true, false] {
        let mut fixture = root_fixture(20);
        if operation {
            fixture.command.operation_id = origin.operation_id;
        } else {
            fixture.command.decision_id = origin.decision_id;
        }
        let second = from_group(&fixture.capture(), &crate::effect_support::empty_history());
        assert!(list(vec![first.clone(), second]).is_err());
    }
}

#[test]
fn administrative_and_judicial_owners_share_one_operation_namespace_across_page_items() {
    let first = administrative(&initial(10), 10, false);
    let mut fixture = root_fixture(20);
    let owner = &first.record_history.records.administrative[0].origin;
    fixture.command.operation_id =
        MeasureDecisionOperationId::from_uuid(owner.operation_id.as_uuid());
    let second = from_group(&fixture.capture(), &crate::effect_support::empty_history());
    assert!(list(vec![first, second]).is_err());
}

#[test]
fn hidden_sibling_revision_cannot_disagree_with_an_administrative_page_item_owner() {
    let initial = initial(10);
    let first = administrative(&initial, 10, false);
    let previous = &initial.record_history.records.judicial.groups[0].capture;
    let mut later = crate::effect_support::LaterFixture::confirm(previous);
    let sibling = id(3020);
    later.effects(vec![
        MeasureEffect::Confirm {
            previous: initial.reference,
        },
        MeasureEffect::Impose(MeasureProposal {
            id: sibling,
            values: previous.measures[0].result.values.clone(),
        }),
    ]);
    later
        .request
        .material
        .result_sources
        .push(MeasureResultSources {
            id: sibling,
            sources: previous.measures[0].result.sources.clone(),
        });
    let history = later.evidence.clone();
    let group = later.capture();
    let index = group
        .measures
        .iter()
        .position(|m| m.result.id == sibling)
        .unwrap();
    let second = member(&group, &history, index);
    assert!(list(vec![first, second]).is_err());
}

#[test]
fn genuine_g2_owners_are_compared_across_full_page_closures() {
    let first = mixed(10).pop().unwrap();
    let origin = &first.record_history.decisions[0].origin;
    let mut fixture = FixtureV2::initial(root_fixture(20));
    fixture.command.operation_id = origin.operation_id;
    fixture.command.decision_id = origin.decision_id;
    let group = fixture.capture();
    let second = MeasureRecordDetail {
        case_id: group.review.case_id,
        reference: reference_v2(&group.measures[0]),
        record: owned_v2(&group, 0),
        record_history: append_v2(&fixture.history, &group),
    };
    assert!(list(vec![first, second]).is_err());
}

#[test]
fn individually_valid_records_must_agree_on_full_shared_sources() {
    let first = initial(10);
    for mutation in 0..4 {
        let mut fixture = root_fixture(20);
        let sources = &mut fixture.material.result_sources[0].sources;
        match mutation {
            0 => sources.subject.changed_by.email = "different-subject@example.test".into(),
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
        let second = from_group(&fixture.capture(), &crate::effect_support::empty_history());
        assert!(list(vec![first.clone(), second]).is_err());
    }
}
