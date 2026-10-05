use super::*;

#[test]
fn oversized_ready_owner_and_nested_row_collections_are_rejected_before_admission() {
    for nested in [false, true] {
        let mut fixture = Fixture::single();
        let groups = &mut fixture
            .material
            .dependency_inventory
            .records
            .records
            .judicial
            .groups;
        if nested {
            groups[0].capture.measures = vec![groups[0].capture.measures[0].clone(); 33];
        } else {
            *groups = vec![groups[0].clone(); 257];
        }
        reject_before_admission(fixture);
    }
}

#[test]
fn all_ready_hearing_prefix_captures_count_before_support_admission() {
    let mut fixture = Fixture::single();
    let hearing = HearingFixture::schedule().capture(None, at());
    let prefix =
        crate::measure_dependency_support::hearing_prefix(vec![hearing], &empty_decision_history());
    fixture.material.dependency_inventory.hearings = vec![prefix; 2];
    for prefix in &mut fixture.material.dependency_inventory.hearings {
        prefix.captures = vec![prefix.captures[0].clone(); 129];
    }
    reject_before_admission(fixture);
}
