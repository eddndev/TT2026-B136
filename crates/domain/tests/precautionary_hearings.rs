use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::hearings::{
    HearingId, HearingKind, HearingModality, HearingNote, HearingParticipantRef, HearingSupportRef,
    HearingTime, HearingVenue,
};
use domain::participants::{ParticipantId, ParticipantRevision};
use domain::precautionary_hearings::{
    MeasureId, MeasureRevision, PrecautionaryHearingId, PrecautionaryHearingOperationId,
    PrecautionaryHearingPurpose, PrecautionaryHearingRevision, PrecautionaryHearingSchedulingBasis,
    PrecautionaryHearingValues, PrecautionaryHearingValuesInput, PrecautionaryMeasureRef,
};
use domain::resource_hearings::{ResourceHearingId, ResourceHearingKind};
use std::any::TypeId;
use time::{macros::datetime, UtcOffset};
use uuid::Uuid;

fn participant(id: u128, revision: u32) -> HearingParticipantRef {
    HearingParticipantRef::new(
        ParticipantId::from_uuid(Uuid::from_u128(id)),
        ParticipantRevision::new(revision).unwrap(),
    )
}

fn support(id: u128, version: u32, digest: u8) -> HearingSupportRef {
    HearingSupportRef::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(id)),
            version: DocumentVersion::new(version).unwrap(),
        },
        Sha256Digest::from_array([digest; 32]),
    )
}

fn target(id: u128, revision: u32, digest: u8) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        MeasureId::from_uuid(Uuid::from_u128(id)),
        MeasureRevision::new(revision).unwrap(),
        Sha256Digest::from_array([digest; 32]),
    )
}

fn basis(
    statement: &str,
    reference: HearingSupportRef,
    locator: &str,
) -> PrecautionaryHearingSchedulingBasis {
    PrecautionaryHearingSchedulingBasis::new(
        HearingNote::new(statement).unwrap(),
        reference,
        HearingNote::new(locator).unwrap(),
    )
}

fn input() -> PrecautionaryHearingValuesInput {
    PrecautionaryHearingValuesInput {
        purpose: PrecautionaryHearingPurpose::Imposition,
        scheduled_at: HearingTime::new(datetime!(1970-01-01 00:00 UTC)).unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new(" Court A ").unwrap(),
        note: None,
        participants: Vec::new(),
        scheduling_basis: basis(" Set by order ", support(77, 4, 0x42), " Page 2 "),
        review_targets: Vec::new(),
    }
}

fn review() -> PrecautionaryHearingValuesInput {
    PrecautionaryHearingValuesInput {
        purpose: PrecautionaryHearingPurpose::Review,
        note: Some(HearingNote::new("Note").unwrap()),
        participants: vec![participant(9, 2)],
        review_targets: vec![target(11, 3, 0x55)],
        ..input()
    }
}

fn canonical(source: PrecautionaryHearingValuesInput) -> Vec<u8> {
    PrecautionaryHearingValues::new(source)
        .unwrap()
        .canonical_bytes()
}

#[test]
fn precautionary_purpose_is_closed_and_does_not_expand_other_hearing_catalogs() {
    for (purpose, tag, name) in [
        (PrecautionaryHearingPurpose::Imposition, 0, "imposition"),
        (PrecautionaryHearingPurpose::Review, 1, "review"),
    ] {
        assert_eq!(purpose.tag(), tag);
        assert_eq!(purpose.as_str(), name);
        assert_eq!(
            name.parse::<PrecautionaryHearingPurpose>().unwrap(),
            purpose
        );
        assert!(name.parse::<HearingKind>().is_err());
        assert!(name.parse::<ResourceHearingKind>().is_err());
    }
    for invalid in [
        "",
        "initial",
        "sentencing",
        "appeal_arguments",
        "Review",
        " review",
    ] {
        assert!(invalid.parse::<PrecautionaryHearingPurpose>().is_err());
    }
}

#[test]
fn values_preserve_exact_schedule_place_participants_and_declaration() {
    let source = review();
    let value = PrecautionaryHearingValues::new(source.clone()).unwrap();
    assert_eq!(value.purpose(), source.purpose);
    assert_eq!(value.scheduled_at(), source.scheduled_at);
    assert_eq!(value.modality(), source.modality);
    assert_eq!(value.venue(), &source.venue);
    assert_eq!(value.note(), source.note.as_ref());
    assert_eq!(value.participants(), source.participants.as_slice());
    assert_eq!(value.scheduling_basis(), &source.scheduling_basis);
    assert_eq!(value.review_targets(), source.review_targets.as_slice());
    let declared = value.scheduling_basis();
    assert_eq!(declared.statement().as_str(), "Set by order");
    assert_eq!(declared.locator().as_str(), "Page 2");
    assert_eq!(declared.support(), support(77, 4, 0x42));
    let selected = &value.review_targets()[0];
    assert_eq!(selected.id().as_uuid(), Uuid::from_u128(11));
    assert_eq!(selected.revision().get(), 3);
    assert_eq!(selected.digest(), Sha256Digest::from_array([0x55; 32]));
}

#[test]
fn scheduling_statement_and_locator_use_required_bounded_notes() {
    for invalid in ["".to_owned(), " \n ".to_owned(), "x".repeat(1001)] {
        assert!(HearingNote::new(&invalid).is_err());
    }
    let mut source = input();
    source.scheduling_basis = basis(&"s".repeat(1000), support(77, 4, 0x42), &"l".repeat(1000));
    let value = PrecautionaryHearingValues::new(source).unwrap();
    assert_eq!(value.scheduling_basis().statement().as_str().len(), 1000);
    assert_eq!(value.scheduling_basis().locator().as_str().len(), 1000);
}

#[test]
fn imposition_has_no_review_targets_and_review_requires_at_least_one() {
    assert!(PrecautionaryHearingValues::new(input()).is_ok());
    let mut invalid_imposition = input();
    invalid_imposition.review_targets.push(target(11, 3, 0x55));
    assert!(PrecautionaryHearingValues::new(invalid_imposition).is_err());
    let mut invalid_review = review();
    invalid_review.review_targets.clear();
    assert!(PrecautionaryHearingValues::new(invalid_review).is_err());
    assert!(PrecautionaryHearingValues::new(review()).is_ok());
}

#[test]
fn review_accepts_32_targets_and_rejects_33_without_truncation() {
    let mut source = review();
    source.review_targets = (1..=32).map(|id| target(id, 1, 0x11)).collect();
    assert_eq!(
        PrecautionaryHearingValues::new(source.clone())
            .unwrap()
            .review_targets()
            .len(),
        32
    );
    source.review_targets.push(target(33, 1, 0x11));
    assert!(PrecautionaryHearingValues::new(source).is_err());
}

#[test]
fn targets_sort_by_uuid_and_reject_duplicate_identity_even_with_different_evidence() {
    let mut source = review();
    source.review_targets = vec![target(9, 2, 1), target(1, 5, 2), target(4, 3, 3)];
    assert_eq!(
        PrecautionaryHearingValues::new(source.clone())
            .unwrap()
            .review_targets(),
        &[target(1, 5, 2), target(4, 3, 3), target(9, 2, 1)]
    );
    let original = canonical(source.clone());
    source.review_targets.reverse();
    assert_eq!(canonical(source), original);
    for duplicate in [
        target(11, 3, 0x55),
        target(11, 4, 0x55),
        target(11, 3, 0x56),
    ] {
        let mut source = review();
        source.review_targets.push(duplicate);
        assert!(PrecautionaryHearingValues::new(source).is_err());
    }
}

#[test]
fn participants_sort_exact_references_and_reject_duplicates_or_more_than_32() {
    let mut source = input();
    assert!(PrecautionaryHearingValues::new(source.clone())
        .unwrap()
        .participants()
        .is_empty());
    source.participants = vec![participant(9, 2), participant(1, 5)];
    assert_eq!(
        PrecautionaryHearingValues::new(source.clone())
            .unwrap()
            .participants(),
        &[participant(1, 5), participant(9, 2)]
    );
    let original = canonical(source.clone());
    source.participants.reverse();
    assert_eq!(canonical(source.clone()), original);
    for duplicate in [participant(1, 5), participant(1, 6)] {
        let mut changed = source.clone();
        changed.participants.push(duplicate);
        assert!(PrecautionaryHearingValues::new(changed).is_err());
    }
    source.participants = (1..=32).map(|id| participant(id, 1)).collect();
    assert_eq!(
        PrecautionaryHearingValues::new(source.clone())
            .unwrap()
            .participants()
            .len(),
        32
    );
    source.participants.push(participant(33, 1));
    assert!(PrecautionaryHearingValues::new(source).is_err());
}

#[test]
fn canonical_bytes_match_an_independent_precautionary_review_vector() {
    let expected = concat!(
        "504845415231",                     // PHEAR1
        "01",                               // Review
        "0000000000000000",                 // Unix epoch
        "00000000",                         // UTC offset
        "00",                               // InPerson
        "00000007436f7572742041",           // Court A
        "01000000044e6f7465",               // Note
        "01",                               // One participant
        "00000000000000000000000000000009", // Participant 9
        "00000002",                         // Participant revision 2
        "0000000c536574206279206f72646572", // Set by order
        "0000000000000000000000000000004d", // Document 77
        "00000004",                         // Exact version 4
        "4242424242424242424242424242424242424242424242424242424242424242",
        "00000006506167652032",             // Page 2
        "01",                               // One review target
        "0000000000000000000000000000000b", // Measure 11
        "00000003",                         // Exact measure revision 3
        "5555555555555555555555555555555555555555555555555555555555555555"
    );
    let bytes = canonical(review());
    let actual: String = bytes.iter().map(|value| format!("{value:02x}")).collect();
    assert_eq!(actual, expected);
    assert!(!bytes.starts_with(b"HEAR1"));
    assert!(!bytes.starts_with(b"RHEAR1"));
    assert_eq!(canonical(input())[6], 0);
}

#[test]
fn canonical_values_commit_each_schedule_declaration_and_target_field() {
    let source = review();
    let original = canonical(source.clone());
    let mutations: &[fn(&mut PrecautionaryHearingValuesInput)] = &[
        |v| {
            v.purpose = PrecautionaryHearingPurpose::Imposition;
            v.review_targets.clear();
        },
        |v| v.scheduled_at = HearingTime::new(datetime!(1970-01-01 00:00:01 UTC)).unwrap(),
        |v| {
            v.scheduled_at = HearingTime::new(
                v.scheduled_at
                    .value()
                    .to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap()),
            )
            .unwrap()
        },
        |v| v.modality = HearingModality::Videoconference,
        |v| v.venue = HearingVenue::new("Court B").unwrap(),
        |v| v.note = None,
        |v| v.note = Some(HearingNote::new("Different note").unwrap()),
        |v| v.participants.clear(),
        |v| v.participants[0] = participant(10, 2),
        |v| v.participants[0] = participant(9, 3),
        |v| v.scheduling_basis = basis("Another order", support(77, 4, 0x42), "Page 2"),
        |v| v.scheduling_basis = basis("Set by order", support(78, 4, 0x42), "Page 2"),
        |v| v.scheduling_basis = basis("Set by order", support(77, 5, 0x42), "Page 2"),
        |v| v.scheduling_basis = basis("Set by order", support(77, 4, 0x43), "Page 2"),
        |v| v.scheduling_basis = basis("Set by order", support(77, 4, 0x42), "Page 3"),
        |v| v.review_targets[0] = target(12, 3, 0x55),
        |v| v.review_targets[0] = target(11, 4, 0x55),
        |v| v.review_targets[0] = target(11, 3, 0x56),
        |v| v.review_targets.push(target(12, 3, 0x55)),
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = source.clone();
        mutate(&mut changed);
        assert_ne!(canonical(changed), original, "mutation {index}");
    }
}

#[test]
fn precautionary_roots_operations_and_measure_ids_remain_separate_types() {
    let id = Uuid::from_u128(81);
    assert_eq!(PrecautionaryHearingId::from_uuid(id).as_uuid(), id);
    assert_eq!(PrecautionaryHearingOperationId::from_uuid(id).as_uuid(), id);
    assert_eq!(MeasureId::from_uuid(id).as_uuid(), id);
    assert_eq!(
        PrecautionaryHearingId::from_uuid(id).to_string(),
        id.to_string()
    );
    assert_ne!(
        TypeId::of::<PrecautionaryHearingId>(),
        TypeId::of::<HearingId>()
    );
    assert_ne!(
        TypeId::of::<PrecautionaryHearingId>(),
        TypeId::of::<ResourceHearingId>()
    );
    assert_ne!(
        TypeId::of::<PrecautionaryHearingId>(),
        TypeId::of::<PrecautionaryHearingOperationId>()
    );
    assert_ne!(
        TypeId::of::<PrecautionaryHearingId>(),
        TypeId::of::<MeasureId>()
    );
    assert_ne!(
        TypeId::of::<PrecautionaryHearingRevision>(),
        TypeId::of::<MeasureRevision>()
    );
}

#[test]
fn both_revision_families_reject_zero_and_exhaust_without_wrapping() {
    assert_eq!(PrecautionaryHearingRevision::initial().get(), 1);
    assert_eq!(
        PrecautionaryHearingRevision::initial()
            .next()
            .unwrap()
            .get(),
        2
    );
    assert!(PrecautionaryHearingRevision::new(0).is_err());
    assert!(PrecautionaryHearingRevision::new(u32::MAX)
        .unwrap()
        .next()
        .is_none());
    assert_eq!(MeasureRevision::initial().get(), 1);
    assert_eq!(MeasureRevision::initial().next().unwrap().get(), 2);
    assert!(MeasureRevision::new(0).is_err());
    assert!(MeasureRevision::new(u32::MAX).unwrap().next().is_none());
    for invalid in ["0", "4294967296", "-1"] {
        assert!(serde_json::from_str::<PrecautionaryHearingRevision>(invalid).is_err());
        assert!(serde_json::from_str::<MeasureRevision>(invalid).is_err());
    }
    assert_eq!(
        serde_json::from_str::<PrecautionaryHearingRevision>("2")
            .unwrap()
            .get(),
        2
    );
    assert_eq!(
        serde_json::from_str::<MeasureRevision>("2").unwrap().get(),
        2
    );
}
