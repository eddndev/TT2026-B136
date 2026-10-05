use super::*;

fn list(
    mut items: Vec<MeasureDecisionRecordReceipt>,
) -> Result<MeasureDecisionRecordPage, ApplicationError> {
    items.sort_by_key(|row| row.origin().decision_id.as_uuid());
    list_result(
        MeasureDecisionReadQuery::default(),
        page(crate::participant_support::case_id(), items),
    )
}

#[test]
fn mixed_page_rejects_cross_family_operations_hidden_decisions_and_member_ownership() {
    let first = v2(10);
    let MeasureDecisionRecordReceipt::V2(original) = &first else {
        unreachable!()
    };
    for mutation in 0..4 {
        let mut fixture = FixtureV2::initial(root_fixture(20));
        match mutation {
            0 => {
                fixture.command.operation_id = original.record_history.records.judicial.groups[0]
                    .origin
                    .operation_id
            }
            1 => {
                fixture.command.operation_id = MeasureDecisionOperationId::from_uuid(
                    original.record_history.records.administrative[0]
                        .origin
                        .operation_id
                        .as_uuid(),
                )
            }
            2 => {
                fixture.command.decision_id = original.record_history.records.judicial.groups[0]
                    .origin
                    .decision_id
            }
            _ => {
                let id = original.record_history.records.judicial.groups[0]
                    .capture
                    .measures[0]
                    .result
                    .id;
                let values = fixture.material.result_sources[0].sources.clone();
                fixture.effects(vec![MeasureEffect::Impose(MeasureProposal {
                    id,
                    values: original.record_history.records.judicial.groups[0]
                        .capture
                        .measures[0]
                        .result
                        .values
                        .clone(),
                })]);
                fixture.material.result_sources = vec![MeasureResultSources {
                    id,
                    sources: values,
                }];
            }
        }
        assert!(list(vec![first.clone(), from_v2(&fixture)]).is_err());
    }
}

#[test]
fn separately_valid_mixed_receipts_must_agree_on_full_shared_source_values() {
    for mutation in 0..4 {
        let first = v1(10);
        let mut fixture = FixtureV2::initial(root_fixture(20));
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
                    .email = "different-bound@example.test".into()
            }
            2 => {
                crate::participant_support::typed_mut(sources.supervisor.as_mut().unwrap())
                    .changed_by
                    .email = "different-participant@example.test".into()
            }
            _ => fixture.material.support.name = "different-original.pdf".into(),
        }
        assert!(list(vec![first, from_v2(&fixture)]).is_err());
    }
}

#[test]
fn a_v2_ancestor_cannot_have_another_complete_capture_under_the_same_owner() {
    let MeasureDecisionRecordReceipt::V2(first) = v2(10) else {
        unreachable!()
    };
    let descendant = from_v2(&FixtureV2::next(
        &first.group,
        &first.record_history,
        20_000,
    ));
    let mut conflict = FixtureV2::initial(root_fixture(20));
    conflict.command.operation_id = first.origin.operation_id;
    conflict.command.decision_id = first.origin.decision_id;
    assert!(list(vec![descendant, from_v2(&conflict)]).is_err());
}
