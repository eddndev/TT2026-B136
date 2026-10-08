#[allow(dead_code, unused_imports)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_correction_capture_support/mod.rs"]
mod correction_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_decision_review_support/mod.rs"]
mod decision_review_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_decision_effect_support/mod.rs"]
mod effect_support;
#[allow(dead_code, unused_imports)]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
mod measure_dependency_support;
#[allow(dead_code, unused_imports)]
mod measure_source_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code, unused_imports)]
mod precautionary_receipt_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_decision_support/mod.rs"]
mod record_decision_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_record_review_support/mod.rs"]
mod record_review_support;
#[allow(dead_code, unused_imports)]
#[path = "measure_record_history_support/mod.rs"]
mod record_support;

use application::measure_corrections::validate_measure_dependency_inventory;
use domain::cases::CaseId;
use measure_dependency_support::*;
use uuid::Uuid;

#[path = "measure_dependency_inventory_support/cycle.rs"]
mod cycle;

#[test]
fn an_empty_supplied_inventory_needs_no_invented_measure_target() {
    let inventory = MeasureAdministrativeDependencyInventory {
        records: empty_decision_history(),
        hearings: vec![],
    };

    validate_measure_dependency_inventory(
        &Hasher,
        CaseId::from_uuid(Uuid::from_u128(1)),
        &inventory,
    )
    .unwrap();
}

#[test]
fn a_real_unanchored_zero_member_group_is_valid_without_any_measure_target() {
    let fixture = Fixture::no_change();
    let case_id = fixture.case_id;
    let group = fixture.capture();
    assert!(group.measures.is_empty());
    assert!(group.review.command.anchor.is_none());

    validate_measure_dependency_inventory(&Hasher, case_id, &judicial_inventory(&group)).unwrap();
}

#[test]
fn a_real_imposition_hearing_prefix_does_not_require_a_measure_owner() {
    let fixture = HearingFixture::schedule();
    let case_id = fixture.case_id;
    let hearing = fixture.capture(None, crate::measure_decision_fixtures::at());
    assert!(hearing.review.resolved_values.review_targets().is_empty());
    let inventory = MeasureAdministrativeDependencyInventory {
        records: empty_decision_history(),
        hearings: vec![hearing_prefix(vec![hearing], &empty_decision_history())],
    };

    validate_measure_dependency_inventory(&Hasher, case_id, &inventory).unwrap();
}

#[test]
fn an_actual_review_and_its_zero_member_anchored_owner_validate_as_a_whole_forest() {
    let base = Fixture::single().capture();
    let initial = judicial_inventory(&base).records;
    let hearing =
        DecisionReviewFixture::schedule(vec![reference(&base.measures[0])], initial.clone())
            .capture(None, base.recorded_at);
    let anchored = anchor_v2(&hearing, &initial, 20);
    let independent = independent_request(90, 91).capture();
    let mut inventory = MeasureAdministrativeDependencyInventory {
        records: append_v2(&initial, &anchored),
        hearings: vec![hearing_prefix(vec![hearing], &initial)],
    };
    inventory.records.records.judicial.groups.extend(
        judicial_inventory(&independent)
            .records
            .records
            .judicial
            .groups,
    );
    inventory.records.records.judicial.groups.reverse();
    assert!(anchored.measures.is_empty());

    validate_measure_dependency_inventory(&Hasher, base.review.case_id, &inventory).unwrap();
    inventory.hearings.clear();
    assert!(
        validate_measure_dependency_inventory(&Hasher, base.review.case_id, &inventory).is_err()
    );
}

#[test]
fn individually_valid_roots_cannot_contradict_the_same_immutable_subject() {
    let base = Fixture::single().capture();
    let mut other = independent_request(1, 80);
    other.material.result_sources[0]
        .sources
        .subject
        .changed_by
        .email = "different-author@example.test".into();
    let other = other.capture();
    let mut inventory = judicial_inventory(&base);
    inventory
        .records
        .records
        .judicial
        .groups
        .extend(judicial_inventory(&other).records.records.judicial.groups);

    assert!(
        validate_measure_dependency_inventory(&Hasher, base.review.case_id, &inventory).is_err()
    );
}

#[test]
fn an_unanchored_zero_member_owner_still_requires_its_exact_scope_and_receipt() {
    let group = Fixture::no_change().capture();
    let mut inventory = judicial_inventory(&group);
    assert!(validate_measure_dependency_inventory(
        &Hasher,
        CaseId::from_uuid(Uuid::from_u128(999)),
        &inventory,
    )
    .is_err());
    inventory.records.records.judicial.groups[0]
        .capture
        .decision
        .actor
        .email = "altered-author@example.test".into();
    assert!(
        validate_measure_dependency_inventory(&Hasher, group.review.case_id, &inventory).is_err()
    );
}

#[test]
fn a_prefix_only_owner_shares_immutable_sources_with_the_anchored_group() {
    let base = Fixture::single().capture();
    let initial = judicial_inventory(&base).records;
    let first =
        DecisionReviewFixture::schedule(vec![reference(&base.measures[0])], initial.clone())
            .capture(None, base.recorded_at);
    let mut replacement = HearingFixture::replace(&first);
    set_context(&mut replacement, first.review.observed_context.clone());
    let second = DecisionReviewFixture {
        hearing: replacement,
        decision_history: initial.clone(),
    }
    .capture(Some(&first), base.recorded_at);
    assert!(second.review.resolved_values.review_targets().is_empty());

    for contradictory in [false, true] {
        let mut request = Fixture::no_change();
        request.command.operation_id = MeasureDecisionOperationId::from_uuid(Uuid::from_u128(9001));
        request.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(9101));
        request.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
            hearing_id: second.review.command.hearing_id,
            revision: second.review.result_revision,
            capture_digest: second.capture_digest,
        });
        request.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
            second.clone(),
        )));
        if contradictory {
            request.material.support.name = "contradictory-original-name.pdf".into();
        }
        let group = request.capture();
        assert!(group.measures.is_empty());
        let mut inventory = judicial_inventory(&group);
        inventory
            .records
            .records
            .judicial
            .groups
            .extend(initial.records.judicial.groups.clone());
        inventory.hearings = vec![hearing_prefix(
            vec![first.clone(), second.clone()],
            &initial,
        )];

        assert_eq!(
            validate_measure_dependency_inventory(&Hasher, base.review.case_id, &inventory)
                .is_err(),
            contradictory,
        );
    }
}
