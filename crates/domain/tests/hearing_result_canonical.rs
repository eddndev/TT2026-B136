mod hearing_result_support;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::hearing_results::*;
use domain::participants::{ParticipantId, ParticipantRevision};
use hearing_result_support::*;
use time::{Duration, UtcOffset};
use uuid::Uuid;

#[test]
fn hres1_matches_six_independent_python_vectors_byte_for_byte() {
    for row in rows() {
        let bytes = canonical(fixture_input(&row["input"]));
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            bytes.len(),
            row["bytes"].as_u64().unwrap() as usize,
            "{}",
            row["name"]
        );
        assert_eq!(hex, row["hex"].as_str().unwrap(), "{}", row["name"]);
    }
}

#[test]
fn exact_minimum_and_maximum_include_utf8_and_all_bounded_collections() {
    let rows = rows();
    assert_eq!(MIN_HEARING_RESULT_CANONICAL_BYTES, 26);
    assert_eq!(MAX_HEARING_RESULT_CANONICAL_BYTES, 146933);
    assert_eq!(
        canonical(fixture_input(&rows[0]["input"])).len(),
        MIN_HEARING_RESULT_CANONICAL_BYTES
    );
    assert_eq!(
        canonical(fixture_input(&rows[5]["input"])).len(),
        MAX_HEARING_RESULT_CANONICAL_BYTES
    );
}

#[test]
fn attendee_permutations_normalize_but_agreement_order_is_preserved() {
    let mut first = fixture_input(&rows()[1]["input"]);
    first.attendees.push(attendee(5, 1));
    let expected = canonical(first.clone());
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut variant = first.clone();
        variant.attendees = order.iter().map(|i| first.attendees[*i].clone()).collect();
        assert_eq!(canonical(variant), expected);
    }
    let mut reordered = first.clone();
    reordered.agreements.reverse();
    assert_ne!(canonical(reordered), expected);
    first.summary = HearingResultText::new(" Declared\r\nResult \u{e1} ").unwrap();
    assert_eq!(canonical(first), expected);
}

#[test]
fn occurrence_extent_time_precision_summary_and_source_kind_are_bound() {
    let base = fixture_input(&rows()[1]["input"]);
    let expected = canonical(base.clone());
    let mut variants = vec![];
    let mut value = base.clone();
    value.extent = HearingResultExtent::Unspecified;
    value.occurrence = HearingResultOccurrence::NotStarted;
    variants.push(value);
    let mut value = base.clone();
    value.extent = HearingResultExtent::Concluded;
    variants.push(value);
    let mut value = base.clone();
    value.event_time = DeclaredHearingResultTime::instant(
        base.event_time.instant_value().unwrap() + Duration::seconds(1),
    )
    .unwrap();
    variants.push(value);
    let mut value = base.clone();
    value.event_time = DeclaredHearingResultTime::instant(
        base.event_time
            .instant_value()
            .unwrap()
            .to_offset(UtcOffset::UTC),
    )
    .unwrap();
    variants.push(value);
    let mut value = base.clone();
    value.event_time =
        DeclaredHearingResultTime::date(base.event_time.local_date(), base.event_time.offset())
            .unwrap();
    variants.push(value);
    let mut value = base.clone();
    value.summary = HearingResultText::new("Declared\nResult a\u{301}").unwrap();
    variants.push(value);
    let mut value = base.clone();
    value.provenance = HearingResultProvenance::new(
        HearingResultProvenanceKind::WrittenRecord,
        base.provenance.reference().cloned(),
        base.provenance.support(),
    )
    .unwrap();
    variants.push(value);
    let mut value = base.clone();
    value.provenance = HearingResultProvenance::new(
        base.provenance.kind(),
        Some(HearingResultReference::new("Other locator").unwrap()),
        base.provenance.support(),
    )
    .unwrap();
    variants.push(value);
    for value in variants {
        assert_ne!(canonical(value), expected);
    }
}

#[test]
fn attendee_identity_revision_capacity_and_observation_are_bound() {
    let base = fixture_input(&rows()[1]["input"]);
    let expected = canonical(base.clone());
    let selected = &base.attendees[0];
    for replacement in [
        HearingResultAttendee::new(
            ParticipantId::from_uuid(Uuid::from_u128(99)),
            selected.revision(),
            selected.capacity().clone(),
            selected.observation().cloned(),
        ),
        HearingResultAttendee::new(
            selected.participant_id(),
            ParticipantRevision::new(8).unwrap(),
            selected.capacity().clone(),
            selected.observation().cloned(),
        ),
        HearingResultAttendee::new(
            selected.participant_id(),
            selected.revision(),
            HearingResultCapacity::new("Other capacity").unwrap(),
            selected.observation().cloned(),
        ),
        HearingResultAttendee::new(
            selected.participant_id(),
            selected.revision(),
            selected.capacity().clone(),
            None,
        ),
        HearingResultAttendee::new(
            selected.participant_id(),
            selected.revision(),
            selected.capacity().clone(),
            Some(HearingResultObservation::new("Other observation").unwrap()),
        ),
    ] {
        let mut variant = base.clone();
        variant.attendees[0] = replacement;
        assert_ne!(canonical(variant), expected);
    }
    let mut empty = base.clone();
    empty.attendees.clear();
    assert_ne!(canonical(empty), expected);
}

#[test]
fn agreement_id_text_and_selection_are_bound() {
    let base = fixture_input(&rows()[1]["input"]);
    let expected = canonical(base.clone());
    for replacement in [
        agreement(99, base.agreements[0].text().as_str()),
        HearingResultAgreement::new(
            base.agreements[0].id(),
            HearingResultText::new("Changed agreement").unwrap(),
        ),
    ] {
        let mut variant = base.clone();
        variant.agreements[0] = replacement;
        assert_ne!(canonical(variant), expected);
    }
    let mut empty = base.clone();
    empty.agreements.clear();
    assert_ne!(canonical(empty), expected);
}

#[test]
fn optional_locator_and_support_identity_version_digest_are_bound() {
    let mut base = fixture_input(&rows()[1]["input"]);
    base.provenance = HearingResultProvenance::new(
        HearingResultProvenanceKind::OperatorNote,
        base.provenance.reference().cloned(),
        base.provenance.support(),
    )
    .unwrap();
    let expected = canonical(base.clone());
    let support = base.provenance.support().unwrap();
    let reference = support.reference();
    for replacement in [
        None,
        Some(HearingResultSupportRef::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(Uuid::from_u128(99)),
                ..reference
            },
            support.digest(),
        )),
        Some(HearingResultSupportRef::new(
            DocumentVersionRef {
                version: DocumentVersion::new(8).unwrap(),
                ..reference
            },
            support.digest(),
        )),
        Some(HearingResultSupportRef::new(
            reference,
            Sha256Digest::from_array([0x99; 32]),
        )),
    ] {
        let mut variant = base.clone();
        variant.provenance = HearingResultProvenance::new(
            base.provenance.kind(),
            base.provenance.reference().cloned(),
            replacement,
        )
        .unwrap();
        assert_ne!(canonical(variant), expected);
    }
    let mut variant = base.clone();
    variant.provenance =
        HearingResultProvenance::new(base.provenance.kind(), None, base.provenance.support())
            .unwrap();
    assert_ne!(canonical(variant), expected);
}
