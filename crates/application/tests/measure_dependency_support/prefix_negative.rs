use super::*;
use time::Duration;

#[test]
fn precautionary_anchors_require_one_complete_prefix_in_both_group_families() {
    let (selected, original) = reviewed();
    let hearing = &original.hearings[0].captures[0];
    let legacy = anchor_v1(hearing, &original.records.records.judicial, 1);
    let versioned = anchor_v2(hearing, &original.records, 2);
    for family in 0..2 {
        let mut inventory = original.clone();
        if family == 0 {
            inventory.records.records.judicial =
                crate::effect_support::append_history(&original.records.records.judicial, &legacy);
        } else {
            inventory.records = append_v2(&original.records, &versioned);
        }
        assert_eq!(inspect(selected, &inventory).dependants().len(), 2);
        inventory.hearings.clear();
        reject(selected, &inventory);
    }
}

#[test]
fn duplicate_empty_truncated_reordered_and_wrong_origin_prefixes_reject() {
    let (selected, mut original) = reviewed();
    let first = original.hearings[0].captures[0].clone();
    let second = DecisionReviewFixture::replace(&first, vec![selected], original.records.clone())
        .capture(Some(&first), first.recorded_at + Duration::seconds(1));
    original.hearings[0].captures.push(second);
    assert_eq!(inspect(selected, &original).dependants().len(), 2);
    for mutation in 0..6 {
        let mut inventory = original.clone();
        match mutation {
            0 => inventory.hearings.push(inventory.hearings[0].clone()),
            1 => inventory.hearings[0].captures.clear(),
            2 => {
                inventory.hearings[0].captures.remove(0);
            }
            3 => inventory.hearings[0].captures.reverse(),
            4 => inventory.hearings[0].origin.capture_digest = Sha256Digest::from_array([99; 32]),
            _ => {
                let mut shorter = inventory.hearings[0].clone();
                shorter.captures.pop();
                inventory.hearings.push(shorter);
            }
        }
        reject(selected, &inventory);
    }
}

#[test]
fn anchor_and_prefix_must_agree_on_the_full_capture_including_recorded_time() {
    let (selected, mut inventory) = reviewed();
    let mut anchor = inventory.hearings[0].captures[0].clone();
    anchor.recorded_at += Duration::seconds(1);
    refresh_hearing(&mut anchor);
    let group = anchor_v2(&anchor, &inventory.records, 1);
    inventory.records = append_v2(&inventory.records, &group);
    reject(selected, &inventory);
}

#[test]
fn unrelated_imposition_prefixes_require_valid_receipts_and_consistent_sources() {
    let base = Fixture::single().capture();
    let selected = reference(&base.measures[0]);
    for contradiction in [false, true] {
        let mut hearing = HearingFixture::schedule();
        if contradiction {
            hearing.sources.participants[1]
                .bound_subject
                .as_mut()
                .unwrap()
                .changed_by
                .email = "Contradictory historical author".into();
        }
        let mut captured = hearing.capture(None, base.recorded_at);
        if !contradiction {
            captured.capture_digest = Sha256Digest::from_array([99; 32]);
        }
        let mut inventory = judicial_inventory(&base);
        let valid = HearingFixture::schedule().capture(None, base.recorded_at);
        let mut prefix = hearing_prefix(vec![valid], &empty_decision_history());
        prefix.origin.capture_digest = captured.capture_digest;
        prefix.origin.submission_digest = captured.review.submission_digest;
        prefix.origin.review_digest = captured.review.review_digest;
        prefix.captures = vec![captured];
        inventory.hearings.push(prefix);
        reject(selected, &inventory);
    }
}

#[test]
fn different_prefixes_cannot_claim_the_same_hearing_operation() {
    let (selected, mut inventory) = reviewed();
    let mut request = DecisionReviewFixture::schedule(vec![selected], inventory.records.clone());
    request.hearing.command.hearing_id = PrecautionaryHearingId::from_uuid(Uuid::from_u128(999));
    let second = request.capture(None, inventory.hearings[0].captures[0].recorded_at);
    inventory
        .hearings
        .push(hearing_prefix(vec![second], &inventory.records));
    reject(selected, &inventory);
}
