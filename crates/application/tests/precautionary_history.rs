#[allow(dead_code)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
mod precautionary_history_support;
#[allow(dead_code)]
#[path = "precautionary_receipt_support/mod.rs"]
mod precautionary_receipt_support;

use application::precautionary_hearings::*;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::hearings::HearingSupportRef;
use domain::precautionary_hearings::{
    PrecautionaryHearingId, PrecautionaryHearingRevision, PrecautionaryHearingSchedulingBasis,
    PrecautionaryHearingValues,
};
use precautionary_history_support::*;
use precautionary_receipt_support::{at, scheduled, values_input, Fixture, Hasher};
use time::Duration;
use uuid::Uuid;

#[test]
fn origin_retains_every_field_of_the_exact_initial_schedule() {
    let initial = scheduled();
    let origin = precautionary_hearing_origin(&Hasher, &initial).unwrap();
    assert_eq!(origin.case_id, initial.review.case_id);
    assert_eq!(origin.hearing_id, initial.review.command.hearing_id);
    assert_eq!(origin.operation_id, initial.review.command.operation_id);
    assert_eq!(origin.revision, PrecautionaryHearingRevision::initial());
    assert_eq!(origin.submission_digest, initial.review.submission_digest);
    assert_eq!(origin.review_digest, initial.review.review_digest);
    assert_eq!(origin.capture_digest, initial.capture_digest);
}

#[test]
fn valid_replacement_and_cancellation_receipts_cannot_be_used_as_origins() {
    let captures = complete_chain();
    for capture in &captures[1..] {
        precautionary_hearing_receipt_matches(&Hasher, capture).unwrap();
        assert!(precautionary_hearing_origin(&Hasher, capture).is_err());
    }
}

#[test]
fn origin_extraction_validates_the_initial_receipt_before_returning_metadata() {
    for mutation in 0..3 {
        let mut initial = scheduled();
        match mutation {
            0 => initial.capture_digest = Sha256Digest::from_array([99; 32]),
            1 => initial.review.submission_digest = Sha256Digest::from_array([99; 32]),
            _ => {
                initial.review.participants[0].overview.display_name = "Changed source".into();
                rehash(&mut initial);
            }
        }
        assert!(precautionary_hearing_origin(&Hasher, &initial).is_err());
    }
}

#[test]
fn a_single_initial_capture_and_its_ascending_replacement_cancellation_chain_validate() {
    let captures = complete_chain();
    let origin = precautionary_hearing_origin(&Hasher, &captures[0]).unwrap();
    assert_flat_and_adjacent(&captures);
    precautionary_hearing_history_matches(&Hasher, &captures[..1], &origin).unwrap();
    precautionary_hearing_history_matches(&Hasher, &captures[..2], &origin).unwrap();
    precautionary_hearing_history_matches(&Hasher, &captures, &origin).unwrap();
}

#[test]
fn an_origin_does_not_replace_missing_initial_capture_material() {
    let initial = scheduled();
    let origin = precautionary_hearing_origin(&Hasher, &initial).unwrap();
    assert!(precautionary_hearing_history_matches(&Hasher, &[], &origin).is_err());
}

#[test]
fn every_origin_field_must_match_the_first_validated_capture() {
    let captures = complete_chain();
    let mutations: &[fn(&mut PrecautionaryHearingOrigin)] = &[
        |v| v.case_id = CaseId::from_uuid(Uuid::from_u128(99)),
        |v| v.hearing_id = PrecautionaryHearingId::from_uuid(Uuid::from_u128(99)),
        |v| v.operation_id = operation(99),
        |v| v.revision = PrecautionaryHearingRevision::new(2).unwrap(),
        |v| v.submission_digest = Sha256Digest::from_array([99; 32]),
        |v| v.review_digest = Sha256Digest::from_array([99; 32]),
        |v| v.capture_digest = Sha256Digest::from_array([99; 32]),
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut origin = precautionary_hearing_origin(&Hasher, &captures[0]).unwrap();
        mutate(&mut origin);
        assert!(
            precautionary_hearing_history_matches(&Hasher, &captures, &origin).is_err(),
            "origin field {index}"
        );
    }
}

#[test]
fn missing_prefix_skipped_revision_reordered_and_duplicated_captures_reject() {
    let captures = complete_chain();
    let origin = precautionary_hearing_origin(&Hasher, &captures[0]).unwrap();
    for selected in [
        vec![captures[1].clone()],
        captures[1..].to_vec(),
        vec![captures[0].clone(), captures[2].clone()],
        vec![
            captures[1].clone(),
            captures[0].clone(),
            captures[2].clone(),
        ],
        vec![
            captures[0].clone(),
            captures[2].clone(),
            captures[1].clone(),
        ],
        vec![
            captures[0].clone(),
            captures[1].clone(),
            captures[1].clone(),
        ],
    ] {
        assert!(precautionary_hearing_history_matches(&Hasher, &selected, &origin).is_err());
    }
}

#[test]
fn self_consistent_foreign_case_or_hearing_captures_cannot_join_the_chain() {
    let captures = complete_chain();
    let origin = precautionary_hearing_origin(&Hasher, &captures[0]).unwrap();
    for change_case in [true, false] {
        let mut foreign = captures[1].clone();
        if change_case {
            remap_case(&mut foreign, CaseId::from_uuid(Uuid::from_u128(99)));
        } else {
            foreign.review.command.hearing_id =
                PrecautionaryHearingId::from_uuid(Uuid::from_u128(99));
            rehash(&mut foreign);
        }
        precautionary_hearing_receipt_matches(&Hasher, &foreign).unwrap();
        assert!(precautionary_hearing_history_matches(
            &Hasher,
            &[captures[0].clone(), foreign],
            &origin
        )
        .is_err());
    }
}

#[test]
fn every_capture_receipt_is_verified_including_first_middle_and_last() {
    let captures = complete_chain();
    let origin = precautionary_hearing_origin(&Hasher, &captures[0]).unwrap();
    for index in 0..captures.len() {
        let mut changed = captures.clone();
        changed[index].capture_digest = Sha256Digest::from_array([99; 32]);
        assert!(
            precautionary_hearing_history_matches(&Hasher, &changed, &origin).is_err(),
            "capture {index}"
        );
    }
}

#[test]
fn valid_flat_receipts_still_require_the_exact_predecessor_digest() {
    let captures = complete_chain();
    let origin = precautionary_hearing_origin(&Hasher, &captures[0]).unwrap();
    let mut changed = captures[1].clone();
    let PrecautionaryHearingChange::Replace {
        expected_capture_digest,
        ..
    } = &mut changed.review.command.change
    else {
        unreachable!()
    };
    *expected_capture_digest = Sha256Digest::from_array([99; 32]);
    rehash(&mut changed);
    precautionary_hearing_receipt_matches(&Hasher, &changed).unwrap();
    assert!(precautionary_hearing_history_matches(
        &Hasher,
        &[captures[0].clone(), changed],
        &origin
    )
    .is_err());
}

#[test]
fn operation_ids_cannot_recur_nonadjacently_anywhere_in_history() {
    for later_operations in [vec![31, 30], vec![31, 33, 31]] {
        let mut captures = vec![scheduled()];
        let origin = precautionary_hearing_origin(&Hasher, &captures[0]).unwrap();
        for id in later_operations {
            let next = replacement(captures.last().unwrap(), id);
            captures.push(next);
        }
        assert_flat_and_adjacent(&captures);
        assert!(precautionary_hearing_history_matches(&Hasher, &captures, &origin).is_err());
    }
}

#[test]
fn cancellation_is_terminal_even_for_a_self_consistent_later_receipt() {
    let captures = complete_chain();
    let origin = precautionary_hearing_origin(&Hasher, &captures[0]).unwrap();
    for replace in [false, true] {
        let next = after_cancel(captures.last().unwrap(), replace);
        precautionary_hearing_receipt_matches(&Hasher, &next).unwrap();
        let mut extended = captures.clone();
        extended.push(next);
        assert!(precautionary_hearing_history_matches(&Hasher, &extended, &origin).is_err());
    }
}

#[test]
fn returning_participant_revision_must_retain_its_original_provenance() {
    for change_provenance in [false, true] {
        let initial = scheduled();
        let origin = precautionary_hearing_origin(&Hasher, &initial).unwrap();
        let mut removal = Fixture::replace(&initial);
        let mut input = values_input();
        input
            .participants
            .retain(|reference| reference.id().as_uuid() != Uuid::from_u128(10));
        let PrecautionaryHearingChange::Replace { values, .. } = &mut removal.command.change else {
            unreachable!()
        };
        *values = PrecautionaryHearingValues::new(input).unwrap();
        removal
            .sources
            .participants
            .retain(|source| source.id().as_uuid() != Uuid::from_u128(10));
        let middle = removal.capture(Some(&initial), at() + Duration::seconds(2));
        let mut return_source = Fixture::replace(&middle);
        return_source.command.operation_id = operation(33);
        if change_provenance {
            participant_support::manual_mut(&mut return_source.sources.participants[0])
                .changed_by
                .email = "different-author@example.test".into();
        }
        let latest = return_source.capture(Some(&middle), at() + Duration::seconds(4));
        let captures = [initial, middle, latest];
        assert_flat_and_adjacent(&captures);
        let result = precautionary_hearing_history_matches(&Hasher, &captures, &origin);
        assert_eq!(result.is_err(), change_provenance);
    }
}

#[test]
fn returning_document_version_must_retain_its_original_metadata() {
    for change_name in [false, true] {
        let initial = scheduled();
        let origin = precautionary_hearing_origin(&Hasher, &initial).unwrap();
        let mut reselection = Fixture::replace(&initial);
        let selected = HearingSupportRef::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(Uuid::from_u128(91)),
                version: DocumentVersion::initial(),
            },
            Sha256Digest::from_array([91; 32]),
        );
        let mut input = values_input();
        input.scheduling_basis = PrecautionaryHearingSchedulingBasis::new(
            input.scheduling_basis.statement().clone(),
            selected,
            input.scheduling_basis.locator().clone(),
        );
        let PrecautionaryHearingChange::Replace { values, .. } = &mut reselection.command.change
        else {
            unreachable!()
        };
        *values = PrecautionaryHearingValues::new(input).unwrap();
        reselection.sources.support.reference = selected.reference();
        reselection.sources.support.digest = selected.digest();
        reselection.sources.support.name = "alternate.pdf".into();
        let middle = reselection.capture(Some(&initial), at() + Duration::seconds(2));
        let mut return_source = Fixture::replace(&middle);
        return_source.command.operation_id = operation(33);
        if change_name {
            return_source.sources.support.name = "different-retained-name.pdf".into();
        }
        let latest = return_source.capture(Some(&middle), at() + Duration::seconds(4));
        let captures = [initial, middle, latest];
        assert_flat_and_adjacent(&captures);
        let result = precautionary_hearing_history_matches(&Hasher, &captures, &origin);
        assert_eq!(result.is_err(), change_name);
    }
}
