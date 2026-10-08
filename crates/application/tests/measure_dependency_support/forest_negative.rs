use super::*;

#[test]
fn malformed_disconnected_owners_reject_even_outside_the_selected_target_ancestry() {
    let base = Fixture::single().capture();
    let selected = reference(&base.measures[0]);
    let other = independent_request(1, 80).capture();
    let other_history = judicial_inventory(&other).records.records.judicial;
    let correction =
        CorrectionFixture::from_group(&other, &crate::effect_support::empty_history(), id(80));
    let administrative = correction.capture();
    let versioned_request = FixtureV2::initial(independent_request(2, 90));
    let versioned = versioned_request.capture();
    let mut valid = judicial_inventory(&base);
    valid
        .records
        .records
        .judicial
        .groups
        .extend(other_history.groups);
    let correction_history = MeasureRecordHistoryEvidence {
        judicial: correction.history.clone(),
        administrative: vec![],
    };
    valid.records.records.administrative =
        crate::record_support::append_administrative(&correction_history, &administrative)
            .administrative;
    valid.records.decisions = append_v2(&versioned_request.history, &versioned).decisions;
    assert!(inspect(selected, &valid).dependants().is_empty());
    for mutation in 0..5 {
        let mut inventory = valid.clone();
        match mutation {
            0 => {
                inventory.records.records.judicial.groups[1]
                    .origin
                    .group_digest = Sha256Digest::from_array([99; 32])
            }
            1 => {
                inventory.records.decisions[0].capture.measures[0]
                    .result
                    .projection
                    .subject
                    .display_name = "Invented member".into()
            }
            2 => {
                inventory.records.records.administrative[0].capture.records[0]
                    .result
                    .projection
                    .subject
                    .display_name = "Invented correction".into()
            }
            3 => inventory.records.records.judicial.groups[1]
                .capture
                .measures
                .clear(),
            _ => {
                inventory.records.decisions[0].capture.review.case_id =
                    CaseId::from_uuid(Uuid::from_u128(999))
            }
        }
        reject(selected, &inventory);
    }
}

#[test]
fn a_disconnected_later_owner_still_requires_its_actual_parent() {
    let base = Fixture::single().capture();
    let selected = reference(&base.measures[0]);
    let other = independent_request(1, 80).capture();
    let later_fixture = crate::effect_support::LaterFixture::confirm(&other);
    let later = later_fixture.clone().capture();
    let complete = crate::effect_support::append_history(&later_fixture.evidence, &later);
    let mut inventory = judicial_inventory(&base);
    inventory
        .records
        .records
        .judicial
        .groups
        .push(complete.groups.last().unwrap().clone());
    reject(selected, &inventory);
}

#[test]
fn duplicate_owners_and_cross_family_operations_reject_in_the_whole_forest() {
    let base = Fixture::single().capture();
    let selected = reference(&base.measures[0]);
    let mut duplicate = judicial_inventory(&base);
    duplicate
        .records
        .records
        .judicial
        .groups
        .push(duplicate.records.records.judicial.groups[0].clone());
    reject(selected, &duplicate);
    let mut request = FixtureV2::initial(independent_request(1, 80));
    request.command.operation_id = base.review.command.operation_id;
    let versioned = request.capture();
    let mut inventory = judicial_inventory(&base);
    inventory.records.decisions = append_v2(&request.history, &versioned).decisions;
    reject(selected, &inventory);
}

#[test]
fn independently_valid_roots_cannot_disagree_about_one_historical_subject() {
    let base = Fixture::single().capture();
    let selected = reference(&base.measures[0]);
    let mut other = independent_request(1, 80);
    other.material.result_sources[0]
        .sources
        .subject
        .changed_by
        .email = "Different immutable author".into();
    let other = other.capture();
    let mut inventory = judicial_inventory(&base);
    inventory
        .records
        .records
        .judicial
        .groups
        .extend(judicial_inventory(&other).records.records.judicial.groups);
    reject(selected, &inventory);
}

#[test]
fn an_unrelated_zero_row_group_is_validated_instead_of_skipped() {
    let base = Fixture::single().capture();
    let selected = reference(&base.measures[0]);
    let mut request = FixtureV2::initial(Fixture::no_change());
    request.identities(4);
    let group = request.capture();
    let mut inventory = judicial_inventory(&base);
    inventory.records.decisions = append_v2(&request.history, &group).decisions;
    assert!(inspect(selected, &inventory).dependants().is_empty());
    inventory.records.decisions[0].capture.decision.actor.email = "Invented captured actor".into();
    reject(selected, &inventory);
}
