use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::hearings::{
    HearingKind, HearingModality, HearingNote, HearingParticipantRef, HearingSupportRef,
    HearingTime, HearingVenue,
};
use domain::participants::{ParticipantId, ParticipantRevision};
use domain::procedural_resources::{ResourceKind, ResourceMode};
use domain::resource_hearings::{
    ResourceHearingKind, ResourceHearingSchedulingBasis, ResourceHearingValues,
    ResourceHearingValuesInput,
};
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

fn basis(statement: &str, reference: HearingSupportRef) -> ResourceHearingSchedulingBasis {
    ResourceHearingSchedulingBasis::new(HearingNote::new(statement).unwrap(), reference)
}

fn input() -> ResourceHearingValuesInput {
    ResourceHearingValuesInput {
        kind: ResourceHearingKind::AppealArguments,
        scheduled_at: HearingTime::new(datetime!(1970-01-01 00:00 UTC)).unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new(" Court A ").unwrap(),
        note: None,
        participants: Vec::new(),
        scheduling_basis: basis("Set by order", support(77, 4, 0x42)),
    }
}

#[test]
fn resource_catalog_is_closed_and_does_not_alias_ordinary_hearings() {
    for (kind, tag, name) in [
        (ResourceHearingKind::AppealArguments, 0, "appeal_arguments"),
        (
            ResourceHearingKind::WrittenRevocation,
            1,
            "written_revocation",
        ),
    ] {
        assert_eq!(kind.tag(), tag);
        assert_eq!(kind.as_str(), name);
        assert_eq!(name.parse::<ResourceHearingKind>().unwrap(), kind);
        assert!(name.parse::<HearingKind>().is_err());
    }
    for invalid in [
        "",
        "initial",
        "oral_trial",
        "appeal",
        "AppealArguments",
        " appeal_arguments",
    ] {
        assert!(invalid.parse::<ResourceHearingKind>().is_err());
    }
}

#[test]
fn compatibility_requires_the_exact_resource_family_and_written_mode() {
    for hearing in [
        ResourceHearingKind::AppealArguments,
        ResourceHearingKind::WrittenRevocation,
    ] {
        for resource in [ResourceKind::Appeal, ResourceKind::Revocation] {
            for mode in [ResourceMode::Oral, ResourceMode::Written] {
                let expected = matches!(
                    (hearing, resource, mode),
                    (
                        ResourceHearingKind::AppealArguments,
                        ResourceKind::Appeal,
                        ResourceMode::Written
                    ) | (
                        ResourceHearingKind::WrittenRevocation,
                        ResourceKind::Revocation,
                        ResourceMode::Written
                    )
                );
                assert_eq!(hearing.is_compatible_with(resource, mode), expected);
            }
        }
    }
}

#[test]
fn values_preserve_scheduling_fields_and_an_explicit_empty_participant_selection() {
    let mut source = input();
    source.note = Some(HearingNote::new("Operator note").unwrap());
    let value = ResourceHearingValues::new(source.clone()).unwrap();
    assert_eq!(value.kind(), source.kind);
    assert_eq!(value.scheduled_at(), source.scheduled_at);
    assert_eq!(value.modality(), source.modality);
    assert_eq!(value.venue(), &source.venue);
    assert_eq!(value.note(), source.note.as_ref());
    assert!(value.participants().is_empty());
    assert_eq!(value.scheduling_basis(), &source.scheduling_basis);
}

#[test]
fn mandatory_scheduling_basis_preserves_its_exact_document_version_and_digest() {
    let reference = support(77, 4, 0x42);
    let value = ResourceHearingValues::new(input()).unwrap();
    let captured = value.scheduling_basis();
    assert_eq!(captured.statement().as_str(), "Set by order");
    assert_eq!(captured.support(), reference);
    assert_eq!(
        captured.support().reference().id.as_uuid(),
        Uuid::from_u128(77)
    );
    assert_eq!(captured.support().reference().version.get(), 4);
    assert_eq!(
        captured.support().digest(),
        Sha256Digest::from_array([0x42; 32])
    );
}

#[test]
fn participants_are_sorted_but_same_identity_is_rejected_even_at_another_revision() {
    let mut source = input();
    source.participants = vec![participant(9, 2), participant(1, 5), participant(4, 3)];
    assert_eq!(
        ResourceHearingValues::new(source).unwrap().participants(),
        &[participant(1, 5), participant(4, 3), participant(9, 2)]
    );
    for repeated in [participant(1, 1), participant(1, 2)] {
        let mut source = input();
        source.participants = vec![participant(1, 1), participant(5, 1), repeated];
        assert!(ResourceHearingValues::new(source).is_err());
    }
}

#[test]
fn participants_accept_32_and_reject_33_without_dropping_any_selection() {
    let mut source = input();
    source.participants = (1..=32).map(|id| participant(id, 1)).collect();
    assert_eq!(
        ResourceHearingValues::new(source.clone())
            .unwrap()
            .participants()
            .len(),
        32
    );
    source.participants.push(participant(33, 1));
    assert!(ResourceHearingValues::new(source).is_err());
}

#[test]
fn canonical_bytes_match_an_independent_resource_hearing_vector() {
    let expected = concat!(
        "524845415231",                     // RHEAR1
        "00",                               // AppealArguments
        "0000000000000000",                 // Unix epoch
        "00000000",                         // UTC offset
        "00",                               // InPerson
        "00000007436f7572742041",           // Court A
        "00",                               // No optional note
        "00",                               // No selected participants
        "0000000c536574206279206f72646572", // Set by order
        "0000000000000000000000000000004d", // Document 77
        "00000004",                         // Exact version 4
        "4242424242424242424242424242424242424242424242424242424242424242"
    );
    let bytes = ResourceHearingValues::new(input())
        .unwrap()
        .canonical_bytes();
    let actual: String = bytes.iter().map(|value| format!("{value:02x}")).collect();
    assert_eq!(actual, expected);
    assert!(!bytes.starts_with(b"HEAR1"));
}

#[test]
fn participant_input_order_is_normalized_but_exact_revision_is_committed() {
    let mut source = input();
    source.participants = vec![participant(9, 2), participant(1, 5)];
    let original = ResourceHearingValues::new(source.clone())
        .unwrap()
        .canonical_bytes();
    source.participants.reverse();
    assert_eq!(
        ResourceHearingValues::new(source.clone())
            .unwrap()
            .canonical_bytes(),
        original
    );
    source.participants[0] = participant(1, 6);
    assert_ne!(
        ResourceHearingValues::new(source)
            .unwrap()
            .canonical_bytes(),
        original
    );
}

#[test]
fn canonical_values_commit_every_field_including_offset_and_scheduling_support() {
    let source = input();
    let original = ResourceHearingValues::new(source.clone())
        .unwrap()
        .canonical_bytes();
    let mut alternatives = Vec::new();
    let mut changed = source.clone();
    changed.kind = ResourceHearingKind::WrittenRevocation;
    alternatives.push(changed);
    let mut changed = source.clone();
    changed.scheduled_at = HearingTime::new(datetime!(1970-01-01 00:00:01 UTC)).unwrap();
    alternatives.push(changed);
    let mut changed = source.clone();
    changed.scheduled_at = HearingTime::new(
        source
            .scheduled_at
            .value()
            .to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap()),
    )
    .unwrap();
    alternatives.push(changed);
    let mut changed = source.clone();
    changed.modality = HearingModality::Videoconference;
    alternatives.push(changed);
    let mut changed = source.clone();
    changed.venue = HearingVenue::new("Court B").unwrap();
    alternatives.push(changed);
    let mut changed = source.clone();
    changed.note = Some(HearingNote::new("Operator note").unwrap());
    alternatives.push(changed);
    let mut changed = source.clone();
    changed.participants.push(participant(9, 2));
    alternatives.push(changed);
    for (statement, reference) in [
        ("Another order", support(77, 4, 0x42)),
        ("Set by order", support(78, 4, 0x42)),
        ("Set by order", support(77, 5, 0x42)),
        ("Set by order", support(77, 4, 0x43)),
    ] {
        let mut changed = source.clone();
        changed.scheduling_basis = basis(statement, reference);
        alternatives.push(changed);
    }
    for changed in alternatives {
        assert_ne!(
            ResourceHearingValues::new(changed)
                .unwrap()
                .canonical_bytes(),
            original
        );
    }
}
