use crate::measure_dependency_support::*;
use uuid::Uuid;

#[test]
fn disconnected_valid_owners_and_shared_sources_do_not_create_target_dependencies() {
    let base = Fixture::single().capture();
    let independent = independent_request(1, 90).capture();
    let independent_v2 = FixtureV2::initial(independent_request(2, 100)).capture();
    let mut inventory = judicial_inventory(&base);
    inventory.records.records.judicial.groups.extend(
        crate::effect_support::append_history(
            &crate::effect_support::empty_history(),
            &independent,
        )
        .groups,
    );
    inventory
        .records
        .decisions
        .extend(append_v2(&empty_decision_history(), &independent_v2).decisions);
    inventory.records.records.judicial.groups.reverse();

    assert_eq!(
        base.measures[0].result.sources,
        independent.measures[0].result.sources
    );
    assert_eq!(
        base.measures[0].result.sources,
        independent_v2.measures[0].result.sources
    );
    assert!(inspect(reference(&base.measures[0]), &inventory)
        .dependants()
        .is_empty());
    assert!(inspect(reference(&independent.measures[0]), &inventory)
        .dependants()
        .is_empty());
    assert!(
        inspect(reference_v2(&independent_v2.measures[0]), &inventory)
            .dependants()
            .is_empty()
    );
}

#[test]
fn standalone_zero_row_owners_are_valid_forest_roots_without_fabricated_members() {
    let base = Fixture::single().capture();
    let mut request = Fixture::no_change();
    request.command.operation_id = MeasureDecisionOperationId::from_uuid(Uuid::from_u128(9001));
    request.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(9101));
    let zero_v1 = request.capture();
    let mut request = FixtureV2::initial(Fixture::no_change());
    request.identities(90);
    let zero_v2 = request.capture();
    let mut inventory = judicial_inventory(&base);
    inventory.records.records.judicial.groups.extend(
        crate::effect_support::append_history(&crate::effect_support::empty_history(), &zero_v1)
            .groups,
    );
    inventory
        .records
        .decisions
        .extend(append_v2(&empty_decision_history(), &zero_v2).decisions);

    assert!(zero_v1.measures.is_empty());
    assert!(zero_v2.measures.is_empty());
    assert!(inspect(reference(&base.measures[0]), &inventory)
        .dependants()
        .is_empty());
}

#[test]
fn report_order_is_deterministic_across_owner_and_hearing_inventory_order() {
    let base = Fixture::single().capture();
    let selected = reference(&base.measures[0]);
    let initial = judicial_inventory(&base).records;
    let low = DecisionReviewFixture::schedule(vec![selected], initial.clone())
        .capture(None, base.recorded_at);
    let mut second = DecisionReviewFixture::schedule(vec![selected], initial.clone());
    second.hearing.command.hearing_id = PrecautionaryHearingId::from_uuid(Uuid::from_u128(50));
    second.hearing.command.operation_id =
        PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(40));
    let high = second.capture(None, base.recorded_at);
    let legacy_low = anchor_v1(&high, &initial.records.judicial, 1);
    let legacy_high = anchor_v1(&low, &initial.records.judicial, 2);
    let versioned = anchor_v2(&low, &initial, 20);
    let later_fixture = crate::effect_support::LaterFixture::confirm(&base);
    let later = later_fixture.clone().capture();
    let mut inventory = MeasureAdministrativeDependencyInventory {
        records: initial.clone(),
        hearings: vec![
            hearing_prefix(vec![high.clone()], &initial),
            hearing_prefix(vec![low.clone()], &initial),
        ],
    };
    for group in [&legacy_high, &later, &legacy_low] {
        let evidence = crate::effect_support::append_history(&initial.records.judicial, group);
        inventory
            .records
            .records
            .judicial
            .groups
            .push(evidence.groups.last().unwrap().clone());
    }
    inventory.records.decisions = append_v2(&initial, &versioned).decisions;
    let expected = vec![
        MeasureAdministrativeDependant::Judicial {
            owner: owner_v1(&later),
            target: selected,
        },
        MeasureAdministrativeDependant::Review {
            hearing: hearing_ref(&low),
            target: selected,
        },
        MeasureAdministrativeDependant::Review {
            hearing: hearing_ref(&high),
            target: selected,
        },
        MeasureAdministrativeDependant::ReviewAnchor {
            owner: owner_v1(&legacy_low),
            hearing: hearing_ref(&high),
            target: selected,
        },
        MeasureAdministrativeDependant::ReviewAnchor {
            owner: owner_v1(&legacy_high),
            hearing: hearing_ref(&low),
            target: selected,
        },
        MeasureAdministrativeDependant::ReviewAnchor {
            owner: owner_v2(&versioned),
            hearing: hearing_ref(&low),
            target: selected,
        },
    ];

    assert_eq!(inspect(selected, &inventory).dependants(), expected);
    inventory.records.records.judicial.groups.reverse();
    inventory.records.decisions.reverse();
    inventory.hearings.reverse();
    assert_eq!(inspect(selected, &inventory).dependants(), expected);
}
