use domain::case_stages::CaseStage;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::hearings::*;
use domain::participants::{ParticipantId, ParticipantRevision};
use domain::DomainError;
use time::macros::datetime;
use uuid::Uuid;

fn participant(id: u128, revision: u32) -> HearingParticipantRef {
    HearingParticipantRef::new(
        ParticipantId::from_uuid(Uuid::from_u128(id)),
        ParticipantRevision::new(revision).unwrap(),
    )
}

fn support() -> HearingSupportRef {
    HearingSupportRef::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(77)),
            version: DocumentVersion::new(4).unwrap(),
        },
        Sha256Digest::from_array([0x42; 32]),
    )
}

fn input() -> HearingValuesInput {
    HearingValuesInput {
        kind: HearingKind::Initial,
        scheduled_at: HearingTime::new(datetime!(2026-09-15 09:00 -6)).unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new(" Court A ").unwrap(),
        note: None,
        participants: vec![],
        conviction_basis: None,
    }
}

#[test]
fn identifiers_preserve_uuid_and_generate_independent_identities() {
    let uuid = Uuid::from_u128(5);
    assert_eq!(HearingId::from_uuid(uuid).as_uuid(), uuid);
    assert_eq!(HearingOperationId::from_uuid(uuid).as_uuid(), uuid);
    assert_eq!(HearingId::from_uuid(uuid).to_string(), uuid.to_string());
    assert_eq!(
        HearingOperationId::from_uuid(uuid).to_string(),
        uuid.to_string()
    );
    assert_ne!(HearingId::new(), HearingId::default());
    assert_ne!(HearingOperationId::new(), HearingOperationId::default());
}

#[test]
fn revisions_reject_zero_and_never_wrap() {
    assert_eq!(
        HearingRevision::new(0),
        Err(DomainError::InvalidHearingRevision)
    );
    assert_eq!(
        HearingRevision::try_from(0),
        Err(DomainError::InvalidHearingRevision)
    );
    assert_eq!(HearingRevision::initial().get(), 1);
    assert_eq!(HearingRevision::initial().next().unwrap().get(), 2);
    assert_eq!(HearingRevision::try_from(9).unwrap().get(), 9);
    assert_eq!(HearingRevision::new(u32::MAX).unwrap().next(), None);
}

#[test]
fn hearing_catalog_has_explicit_tags_and_required_stages() {
    for (kind, tag, name, stage) in [
        (HearingKind::Initial, 0, "initial", CaseStage::Investigation),
        (
            HearingKind::Intermediate,
            1,
            "intermediate",
            CaseStage::Intermediate,
        ),
        (HearingKind::OralTrial, 2, "oral_trial", CaseStage::Trial),
        (HearingKind::Sentencing, 3, "sentencing", CaseStage::Trial),
    ] {
        assert_eq!(kind.tag(), tag);
        assert_eq!(kind.as_str(), name);
        assert_eq!(kind.required_stage(), stage);
        assert_eq!(name.parse::<HearingKind>().unwrap(), kind);
    }
    for invalid in ["", "Initial", " initial", "sentencia", "4"] {
        assert!(invalid.parse::<HearingKind>().is_err());
    }
}

#[test]
fn organizational_status_and_modality_are_strictly_named() {
    for (value, tag, name) in [
        (HearingModality::InPerson, 0, "in_person"),
        (HearingModality::Videoconference, 1, "videoconference"),
    ] {
        assert_eq!(value.tag(), tag);
        assert_eq!(value.as_str(), name);
        assert_eq!(name.parse::<HearingModality>().unwrap(), value);
    }
    for (value, tag, name) in [
        (HearingStatus::Scheduled, 0, "scheduled"),
        (HearingStatus::Cancelled, 1, "cancelled"),
    ] {
        assert_eq!(value.tag(), tag);
        assert_eq!(value.as_str(), name);
        assert_eq!(name.parse::<HearingStatus>().unwrap(), value);
    }
    assert!("held".parse::<HearingStatus>().is_err());
    assert!("past".parse::<HearingStatus>().is_err());
    assert!("video".parse::<HearingModality>().is_err());
}

#[test]
fn venue_trims_without_rewriting_unicode_or_opening_connection_text() {
    assert_eq!(HearingVenue::new(" \u{e1} ").unwrap().as_str(), "\u{e1}");
    assert_eq!(
        HearingVenue::new("https://example.test/x")
            .unwrap()
            .as_str(),
        "https://example.test/x"
    );
    assert_ne!(
        HearingVenue::new("\u{e1}").unwrap(),
        HearingVenue::new("a\u{301}").unwrap()
    );
    assert!(HearingVenue::new(&"\u{1f642}".repeat(500)).is_ok());
    assert!(HearingVenue::new(&"\u{1f642}".repeat(501)).is_err());
    for invalid in [
        "", "   ", "a\nb", "a\r\nb", "a\tb", "a\0b", "\u{7f}a", "a\u{85}",
    ] {
        assert!(HearingVenue::new(invalid).is_err(), "{invalid:?}");
    }
}

#[test]
fn note_normalizes_crlf_accepts_lf_and_limits_unicode_characters() {
    assert_eq!(
        HearingNote::new(" \u{e1}\r\nb \n").unwrap().as_str(),
        "\u{e1}\nb"
    );
    assert!(HearingNote::new(&"\u{1f642}".repeat(1000)).is_ok());
    assert!(HearingNote::new(&"\u{1f642}".repeat(1001)).is_err());
    assert_ne!(
        HearingNote::new("\u{e1}").unwrap(),
        HearingNote::new("a\u{301}").unwrap()
    );
    for invalid in ["", " \r\n ", "a\rb", "a\tb", "\0a", "\u{7f}a", "a\u{85}"] {
        assert!(HearingNote::new(invalid).is_err(), "{invalid:?}");
    }
    assert_eq!(HearingNote::optional(None).unwrap(), None);
    assert_eq!(HearingNote::optional(Some(" \r\n ")).unwrap(), None);
    assert_eq!(
        HearingNote::optional(Some(" n "))
            .unwrap()
            .unwrap()
            .as_str(),
        "n"
    );
    assert!(HearingNote::optional(Some("\t")).is_err());
    assert!(HearingNote::optional(Some(&"n".repeat(1001))).is_err());
}

#[test]
fn references_preserve_exact_identity_revision_and_digest() {
    let reference = participant(4, 9);
    assert_eq!(reference.id().as_uuid(), Uuid::from_u128(4));
    assert_eq!(reference.revision().get(), 9);
    let proof = support();
    assert_eq!(proof.reference().id.as_uuid(), Uuid::from_u128(77));
    assert_eq!(proof.reference().version.get(), 4);
    assert_eq!(proof.digest(), Sha256Digest::from_array([0x42; 32]));
    let basis =
        HearingConvictionBasis::new(HearingNote::new(" Declared conviction ").unwrap(), proof);
    assert_eq!(basis.statement().as_str(), "Declared conviction");
    assert_eq!(basis.support(), proof);
}

#[test]
fn values_preserve_fields_and_allow_an_explicitly_empty_selection() {
    let expected = input();
    let values = HearingValues::new(expected.clone()).unwrap();
    assert_eq!(values.kind(), expected.kind);
    assert_eq!(values.scheduled_at(), expected.scheduled_at);
    assert_eq!(values.modality(), expected.modality);
    assert_eq!(values.venue(), &expected.venue);
    assert_eq!(values.note(), None);
    assert!(values.participants().is_empty());
    assert_eq!(values.conviction_basis(), None);
}

#[test]
fn participant_references_are_sorted_by_uuid_without_discarding_duplicates() {
    let mut values = input();
    values.participants = vec![participant(9, 2), participant(1, 5), participant(4, 3)];
    assert_eq!(
        HearingValues::new(values).unwrap().participants(),
        &[participant(1, 5), participant(4, 3), participant(9, 2)]
    );
    for duplicate in [participant(1, 1), participant(1, 2)] {
        let mut values = input();
        values.participants = vec![participant(1, 1), participant(5, 1), duplicate];
        assert_eq!(
            HearingValues::new(values),
            Err(DomainError::InvalidHearingValue("participants"))
        );
    }
}

#[test]
fn participant_limit_applies_before_canonical_encoding() {
    assert_eq!(MAX_HEARING_PARTICIPANTS, 32);
    let mut values = input();
    values.participants = (1..=32).map(|id| participant(id, 1)).collect();
    assert_eq!(
        HearingValues::new(values.clone())
            .unwrap()
            .participants()
            .len(),
        32
    );
    values.participants.push(participant(33, 1));
    assert_eq!(
        HearingValues::new(values),
        Err(DomainError::InvalidHearingValue("participants"))
    );
}

#[test]
fn conviction_basis_is_required_only_for_individualization() {
    let proof =
        HearingConvictionBasis::new(HearingNote::new("Operator declaration").unwrap(), support());
    for kind in [
        HearingKind::Initial,
        HearingKind::Intermediate,
        HearingKind::OralTrial,
        HearingKind::Sentencing,
    ] {
        let mut values = input();
        values.kind = kind;
        assert_eq!(
            HearingValues::new(values.clone()).is_ok(),
            kind != HearingKind::Sentencing
        );
        values.conviction_basis = Some(proof.clone());
        let result = HearingValues::new(values);
        if kind == HearingKind::Sentencing {
            assert_eq!(result.unwrap().conviction_basis(), Some(&proof));
        } else {
            assert_eq!(
                result,
                Err(DomainError::InvalidHearingValue("conviction_basis"))
            );
        }
    }
}
