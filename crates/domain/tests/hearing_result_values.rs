mod hearing_result_support;
use domain::hearing_results::*;
use domain::DomainError;
use hearing_result_support::*;
use uuid::Uuid;

#[test]
fn identifiers_and_revisions_preserve_identity_and_never_wrap() {
    let uuid = Uuid::from_u128(5);
    assert_eq!(HearingResultId::from_uuid(uuid).as_uuid(), uuid);
    assert_eq!(HearingResultOperationId::from_uuid(uuid).as_uuid(), uuid);
    assert_eq!(HearingResultAgreementId::from_uuid(uuid).as_uuid(), uuid);
    assert_eq!(
        HearingResultId::from_uuid(uuid).to_string(),
        uuid.to_string()
    );
    assert_eq!(
        HearingResultOperationId::from_uuid(uuid).to_string(),
        uuid.to_string()
    );
    assert_eq!(
        HearingResultAgreementId::from_uuid(uuid).to_string(),
        uuid.to_string()
    );
    assert_ne!(HearingResultId::new(), HearingResultId::default());
    assert_ne!(
        HearingResultOperationId::new(),
        HearingResultOperationId::default()
    );
    assert_ne!(
        HearingResultAgreementId::new(),
        HearingResultAgreementId::default()
    );
    assert_eq!(HearingResultRevision::initial().get(), 1);
    assert_eq!(HearingResultRevision::initial().next().unwrap().get(), 2);
    assert_eq!(HearingResultRevision::new(u32::MAX).unwrap().next(), None);
    assert_eq!(
        HearingResultRevision::try_from(0),
        Err(DomainError::InvalidHearingResultRevision)
    );
    assert_eq!(HearingResultRevision::try_from(9).unwrap().get(), 9);
    assert!(serde_json::from_str::<HearingResultRevision>("0").is_err());
    assert_eq!(
        serde_json::from_str::<HearingResultRevision>("7")
            .unwrap()
            .get(),
        7
    );
}

#[test]
fn catalogs_use_explicit_wire_tags_and_reject_unknown_names() {
    for (value, tag, name) in [
        (HearingResultOccurrence::Occurred, 0, "occurred"),
        (HearingResultOccurrence::NotStarted, 1, "not_started"),
    ] {
        assert_eq!((value.tag(), value.as_str()), (tag, name));
        assert_eq!(name.parse::<HearingResultOccurrence>().unwrap(), value);
    }
    for (value, tag, name) in [
        (HearingResultExtent::Partial, 0, "partial"),
        (HearingResultExtent::Concluded, 1, "concluded"),
        (HearingResultExtent::Unspecified, 2, "unspecified"),
    ] {
        assert_eq!((value.tag(), value.as_str()), (tag, name));
        assert_eq!(name.parse::<HearingResultExtent>().unwrap(), value);
    }
    for (value, tag, name) in [
        (
            HearingResultProvenanceKind::OperatorNote,
            0,
            "operator_note",
        ),
        (
            HearingResultProvenanceKind::OralReference,
            1,
            "oral_reference",
        ),
        (
            HearingResultProvenanceKind::WrittenRecord,
            2,
            "written_record",
        ),
    ] {
        assert_eq!((value.tag(), value.as_str()), (tag, name));
        assert_eq!(name.parse::<HearingResultProvenanceKind>().unwrap(), value);
    }
    for (value, tag, name) in [
        (HearingResultStatus::Recorded, 0, "recorded"),
        (HearingResultStatus::Withdrawn, 1, "withdrawn"),
    ] {
        assert_eq!((value.tag(), value.as_str()), (tag, name));
        assert_eq!(name.parse::<HearingResultStatus>().unwrap(), value);
    }
    for invalid in ["", "Occurred", " occurred", "scheduled", "3"] {
        assert!(invalid.parse::<HearingResultOccurrence>().is_err());
        assert!(invalid.parse::<HearingResultExtent>().is_err());
        assert!(invalid.parse::<HearingResultProvenanceKind>().is_err());
        assert!(invalid.parse::<HearingResultStatus>().is_err());
    }
}

#[test]
fn text_normalizes_without_rewriting_unicode_and_counts_scalars() {
    assert_eq!(
        HearingResultText::new(" \u{e1}\r\nb \n").unwrap().as_str(),
        "\u{e1}\nb"
    );
    assert_ne!(
        HearingResultText::new("\u{e1}").unwrap(),
        HearingResultText::new("a\u{301}").unwrap()
    );
    assert!(HearingResultText::new(&"\u{1f642}".repeat(1000)).is_ok());
    assert!(HearingResultText::new(&"\u{1f642}".repeat(1001)).is_err());
    assert!(HearingResultCapacity::new(&"\u{1f642}".repeat(100)).is_ok());
    assert!(HearingResultCapacity::new(&"\u{1f642}".repeat(101)).is_err());
    assert!(HearingResultObservation::new(&"\u{1f642}".repeat(500)).is_ok());
    assert!(HearingResultObservation::new(&"\u{1f642}".repeat(501)).is_err());
    assert!(HearingResultReference::new(&"\u{1f642}".repeat(200)).is_ok());
    assert!(HearingResultReference::new(&"\u{1f642}".repeat(201)).is_err());
    assert_eq!(
        HearingResultReference::new(" https://example.test/a ")
            .unwrap()
            .as_str(),
        "https://example.test/a"
    );
    for invalid in ["", "   ", "a\tb", "a\rb", "\0a", "a\u{85}", "\u{7f}a"] {
        assert!(HearingResultText::new(invalid).is_err());
        assert!(HearingResultCapacity::new(invalid).is_err());
        assert!(HearingResultObservation::new(invalid).is_err());
        assert!(HearingResultReference::new(invalid).is_err());
    }
    for invalid in ["a\nb", "a\r\nb"] {
        assert!(HearingResultCapacity::new(invalid).is_err());
        assert!(HearingResultReference::new(invalid).is_err());
    }
}

#[test]
fn optional_text_distinguishes_blank_from_disallowed_controls() {
    assert_eq!(HearingResultText::optional(None).unwrap(), None);
    assert_eq!(HearingResultText::optional(Some(" \r\n ")).unwrap(), None);
    assert_eq!(
        HearingResultObservation::optional(Some(" \r\n ")).unwrap(),
        None
    );
    assert_eq!(HearingResultReference::optional(Some(" ")).unwrap(), None);
    assert_eq!(
        HearingResultObservation::optional(Some(" a\r\nb "))
            .unwrap()
            .unwrap()
            .as_str(),
        "a\nb"
    );
    for invalid in ["\t", "\0", "\u{85}"] {
        assert!(HearingResultText::optional(Some(invalid)).is_err());
        assert!(HearingResultObservation::optional(Some(invalid)).is_err());
        assert!(HearingResultReference::optional(Some(invalid)).is_err());
    }
}

#[test]
fn provenance_requires_locator_for_external_antecedents_but_support_is_independent() {
    for kind in [
        HearingResultProvenanceKind::OperatorNote,
        HearingResultProvenanceKind::OralReference,
        HearingResultProvenanceKind::WrittenRecord,
    ] {
        assert_eq!(
            HearingResultProvenance::new(kind, None, Some(support())).is_ok(),
            kind == HearingResultProvenanceKind::OperatorNote
        );
        let source = HearingResultProvenance::new(
            kind,
            Some(HearingResultReference::new(" Page 2 ").unwrap()),
            Some(support()),
        )
        .unwrap();
        assert_eq!(source.kind(), kind);
        assert_eq!(source.reference().unwrap().as_str(), "Page 2");
        assert_eq!(source.support(), Some(support()));
        assert!(HearingResultProvenance::new(
            kind,
            Some(HearingResultReference::new("Recording 12:00").unwrap()),
            None
        )
        .is_ok());
    }
}

#[test]
fn exact_references_and_declarations_preserve_independent_fields() {
    let selected = attendee(7, 2);
    assert_eq!(selected.participant_id().as_uuid(), Uuid::from_u128(7));
    assert_eq!(selected.revision().get(), 2);
    assert_eq!(selected.capacity().as_str(), "Counsel");
    assert_eq!(selected.observation(), None);
    let proof = support();
    assert_eq!(proof.reference().id.as_uuid(), Uuid::from_u128(77));
    assert_eq!(proof.reference().version.get(), 4);
    assert_eq!(proof.digest().as_bytes(), &[0x42; 32]);
    let previous = HearingResultContinuationRef::new(
        HearingResultId::from_uuid(Uuid::from_u128(8)),
        HearingResultRevision::new(3).unwrap(),
    );
    assert_eq!(previous.id().as_uuid(), Uuid::from_u128(8));
    assert_eq!(previous.revision().get(), 3);
    let item = agreement(42, " Declared agreement ");
    assert_eq!(item.id().as_uuid(), Uuid::from_u128(42));
    assert_eq!(item.text().as_str(), "Declared agreement");
}

#[test]
fn values_preserve_fields_with_explicit_empty_lists() {
    let expected = input();
    let actual = HearingResultValues::new(expected.clone()).unwrap();
    assert_eq!(actual.occurrence(), expected.occurrence);
    assert_eq!(actual.extent(), expected.extent);
    assert_eq!(actual.event_time(), expected.event_time);
    assert_eq!(actual.summary(), &expected.summary);
    assert_eq!(actual.provenance(), &expected.provenance);
    assert!(actual.attendees().is_empty());
    assert!(actual.agreements().is_empty());
}

#[test]
fn no_start_requires_unspecified_extent_but_allows_attendance_and_agreements() {
    for occurrence in [
        HearingResultOccurrence::Occurred,
        HearingResultOccurrence::NotStarted,
    ] {
        for extent in [
            HearingResultExtent::Partial,
            HearingResultExtent::Concluded,
            HearingResultExtent::Unspecified,
        ] {
            let mut value = input();
            value.occurrence = occurrence;
            value.extent = extent;
            value.attendees.push(attendee(1, 4));
            value.agreements.push(agreement(1, "Next meeting declared"));
            assert_eq!(
                HearingResultValues::new(value).is_ok(),
                occurrence == HearingResultOccurrence::Occurred
                    || extent == HearingResultExtent::Unspecified
            );
        }
    }
}

#[test]
fn attendees_sort_by_uuid_and_reject_same_identity_at_any_revision() {
    let mut value = input();
    value.attendees = vec![attendee(9, 2), attendee(1, 4), attendee(4, 3)];
    assert_eq!(
        HearingResultValues::new(value).unwrap().attendees(),
        &[attendee(1, 4), attendee(4, 3), attendee(9, 2)]
    );
    for revision in [1, 2] {
        let mut value = input();
        value.attendees = vec![attendee(1, 1), attendee(9, 1), attendee(1, revision)];
        assert_eq!(
            HearingResultValues::new(value),
            Err(DomainError::InvalidHearingResultValue("attendees"))
        );
    }
}

#[test]
fn agreements_preserve_order_and_reject_duplicate_ids_even_with_different_text() {
    let mut value = input();
    value.agreements = vec![agreement(9, "first"), agreement(1, "second")];
    assert_eq!(
        HearingResultValues::new(value.clone())
            .unwrap()
            .agreements(),
        value.agreements
    );
    value.agreements.push(agreement(9, "other text"));
    assert_eq!(
        HearingResultValues::new(value),
        Err(DomainError::InvalidHearingResultValue("agreements"))
    );
}

#[test]
fn list_caps_are_enforced_before_counts_can_truncate() {
    assert_eq!(MAX_HEARING_RESULT_ATTENDEES, 32);
    assert_eq!(MAX_HEARING_RESULT_AGREEMENTS, 16);
    let mut value = input();
    value.attendees = (1..=32).map(|id| attendee(id, 1)).collect();
    value.agreements = (1..=16).map(|id| agreement(id, "value")).collect();
    assert!(HearingResultValues::new(value.clone()).is_ok());
    let mut too_many = value.clone();
    too_many.attendees.push(attendee(33, 1));
    assert!(HearingResultValues::new(too_many).is_err());
    value.agreements.push(agreement(17, "value"));
    assert!(HearingResultValues::new(value).is_err());
}
