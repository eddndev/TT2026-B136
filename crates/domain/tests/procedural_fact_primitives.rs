use domain::procedural_facts::{
    FactDeclaration, FactLabel, FactOperationId, FactRevision, FactText, NotificationCharacter,
    NotificationContext, NotificationId, NotificationMedium, NotificationOutcome, ResolutionClass,
    ResolutionId,
};
use domain::DomainError;
use uuid::Uuid;

#[test]
fn identifiers_preserve_exact_uuids_and_generate_distinct_root_and_operation_ids() {
    let uuid = Uuid::from_u128(17);
    let resolution = ResolutionId::from_uuid(uuid);
    let notification = NotificationId::from_uuid(uuid);
    let operation = FactOperationId::from_uuid(uuid);
    assert_eq!(resolution.as_uuid(), uuid);
    assert_eq!(notification.as_uuid(), uuid);
    assert_eq!(operation.as_uuid(), uuid);
    assert_eq!(resolution.to_string(), uuid.to_string());
    assert_eq!(notification.to_string(), uuid.to_string());
    assert_eq!(operation.to_string(), uuid.to_string());
    assert_ne!(ResolutionId::new(), ResolutionId::new());
    assert_ne!(NotificationId::new(), NotificationId::new());
    assert_ne!(FactOperationId::new(), FactOperationId::new());
    assert_eq!(ResolutionId::from_uuid(Uuid::nil()).as_uuid(), Uuid::nil());
    assert_eq!(
        NotificationId::from_uuid(Uuid::nil()).as_uuid(),
        Uuid::nil()
    );
    assert_eq!(
        FactOperationId::from_uuid(Uuid::nil()).as_uuid(),
        Uuid::nil()
    );
}

#[test]
fn revisions_are_positive_and_exhaustion_never_wraps_to_zero() {
    assert_eq!(FactRevision::initial().get(), 1);
    assert_eq!(FactRevision::initial().next().unwrap().get(), 2);
    assert_eq!(FactRevision::new(42).unwrap().get(), 42);
    assert_eq!(FactRevision::new(u32::MAX).unwrap().next(), None);
    assert_eq!(
        FactRevision::new(0),
        Err(DomainError::InvalidProceduralFact("revision"))
    );
}

#[test]
fn labels_trim_outer_space_and_count_unicode_scalars_without_composition() {
    assert_eq!(
        FactLabel::new("  Declared court  ").unwrap().as_str(),
        "Declared court"
    );
    assert!(FactLabel::new(&"\u{1f642}".repeat(200)).is_ok());
    assert_eq!(
        FactLabel::new(&"\u{1f642}".repeat(201)),
        Err(DomainError::InvalidProceduralFact("label"))
    );
    assert_ne!(
        FactLabel::new("\u{e1}").unwrap(),
        FactLabel::new("a\u{301}").unwrap()
    );
    assert_eq!(FactLabel::new(" a  b ").unwrap().as_str(), "a  b");
}

#[test]
fn labels_reject_empty_values_and_all_controls_before_trimming() {
    for value in [
        "", "   ", "\n", "a\nb", "a\r\nb", "a\rb", "\ta", "a\t", "\0a", "a\u{7f}", "\u{85}a",
    ] {
        assert_eq!(
            FactLabel::new(value),
            Err(DomainError::InvalidProceduralFact("label")),
            "{value:?}"
        );
    }
}

#[test]
fn multiline_text_normalizes_crlf_and_outer_space_without_unicode_composition() {
    let value = FactText::new("  First\r\nSecond\nThird \r\n ").unwrap();
    assert_eq!(value.as_str(), "First\nSecond\nThird");
    assert_eq!(value, FactText::new("First\nSecond\nThird").unwrap());
    assert_ne!(
        FactText::new("\u{e1}").unwrap(),
        FactText::new("a\u{301}").unwrap()
    );
    assert_eq!(FactText::new("a\n\nb").unwrap().as_str(), "a\n\nb");
}

#[test]
fn text_scalar_limit_is_one_thousand_after_normalization() {
    assert!(FactText::new(&"\u{1f642}".repeat(1000)).is_ok());
    assert_eq!(
        FactText::new(&"\u{1f642}".repeat(1001)),
        Err(DomainError::InvalidProceduralFact("text"))
    );
    let at_limit = format!("{}\r\nb", "a".repeat(998));
    assert_eq!(
        FactText::new(&at_limit).unwrap().as_str().chars().count(),
        1000
    );
    assert!(FactText::new(&format!("{at_limit}c")).is_err());
}

#[test]
fn multiline_text_rejects_empty_values_and_controls_even_at_trimmed_edges() {
    for value in [
        "", "   ", "\r\n ", "\ta", "a\t", "a\rb", "\ra", "a\r", "\0a", "a\u{7f}", "\u{85}a",
    ] {
        assert_eq!(
            FactText::new(value),
            Err(DomainError::InvalidProceduralFact("text")),
            "{value:?}"
        );
    }
}

#[test]
fn resolution_classes_preserve_other_description_without_deriving_a_class() {
    let values = [
        ResolutionClass::Order,
        ResolutionClass::Judgment,
        ResolutionClass::Other(FactLabel::new("Declared decision").unwrap()),
    ];
    assert_ne!(values[0], values[1]);
    assert_ne!(values[0], values[2]);
    assert_eq!(
        values[2].clone(),
        ResolutionClass::Other(FactLabel::new(" Declared decision ").unwrap())
    );
    let ResolutionClass::Other(label) = &values[2] else {
        panic!("expected other class");
    };
    assert_eq!(label.as_str(), "Declared decision");
}

#[test]
fn notification_character_medium_and_context_are_independent_declarations() {
    let label = FactLabel::new("Declared alternative").unwrap();
    assert_ne!(
        NotificationCharacter::Personal,
        NotificationCharacter::Publication
    );
    assert_ne!(
        NotificationCharacter::Other(label.clone()),
        NotificationCharacter::Other(FactLabel::new("Different declaration").unwrap())
    );
    assert_ne!(NotificationMedium::InPerson, NotificationMedium::Electronic);
    assert_ne!(
        NotificationMedium::Other(label.clone()),
        NotificationMedium::Other(FactLabel::new("Different declaration").unwrap())
    );
    assert_ne!(
        NotificationContext::InHearing,
        NotificationContext::OutsideHearing
    );
    assert_ne!(
        NotificationContext::Other(label.clone()),
        NotificationContext::Other(FactLabel::new("Different declaration").unwrap())
    );
    let declared = (
        NotificationCharacter::Personal,
        NotificationMedium::Electronic,
        NotificationContext::OutsideHearing,
    );
    assert_eq!(declared.0, NotificationCharacter::Personal);
    assert_eq!(declared.1, NotificationMedium::Electronic);
    assert_eq!(declared.2, NotificationContext::OutsideHearing);
}

#[test]
fn attempted_and_practiced_are_distinct_explicit_outcomes() {
    let original = NotificationOutcome::Attempted;
    let copied = original;
    assert_eq!(original, copied);
    assert_ne!(
        NotificationOutcome::Practiced,
        NotificationOutcome::Attempted
    );
}

#[test]
fn declaration_preserves_known_value_or_explicit_unknown_reason() {
    let known = FactDeclaration::Known(NotificationOutcome::Attempted);
    let unknown: FactDeclaration<NotificationOutcome> =
        FactDeclaration::Unknown(FactText::new(" Not stated in the source ").unwrap());
    assert_ne!(known, unknown);
    assert_eq!(
        unknown,
        FactDeclaration::Unknown(FactText::new("Not stated in the source").unwrap())
    );
    assert_ne!(
        unknown,
        FactDeclaration::Unknown(FactText::new("Source not available").unwrap())
    );
    assert_eq!(
        known.clone(),
        FactDeclaration::Known(NotificationOutcome::Attempted)
    );
    let FactDeclaration::Unknown(reason) = unknown else {
        panic!("expected unknown declaration");
    };
    assert_eq!(reason.as_str(), "Not stated in the source");
}
