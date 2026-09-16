mod procedural_fact_support;
use domain::{procedural_facts::*, procedural_time::DeclaredProceduralTime};
use procedural_fact_support::*;
use time::UtcOffset;

#[test]
fn canonical_time_never_equates_missing_precision_offset_or_second() {
    let date = "2026-09-16".parse().unwrap();
    let times = [
        DeclaredProceduralTime::unknown(),
        DeclaredProceduralTime::date(date, None).unwrap(),
        DeclaredProceduralTime::date(date, Some(UtcOffset::UTC)).unwrap(),
        DeclaredProceduralTime::minute(date, 0, 0, None).unwrap(),
        DeclaredProceduralTime::minute(date, 0, 0, Some(UtcOffset::UTC)).unwrap(),
        DeclaredProceduralTime::second(date, 0, 0, 0, None).unwrap(),
        DeclaredProceduralTime::second(date, 0, 0, 0, Some(UtcOffset::UTC)).unwrap(),
    ];
    let encodings: Vec<_> = times
        .into_iter()
        .map(|time| {
            let mut input = resolution_input();
            input.issued_at = time;
            ResolutionValues::new(input).canonical_bytes()
        })
        .collect();
    for (i, first) in encodings.iter().enumerate() {
        assert!(first.starts_with(b"PFRES1"));
        for second in encodings.iter().skip(i + 1) {
            assert_ne!(first, second);
        }
    }
}

#[test]
fn equal_utc_instants_keep_different_declared_local_fields() {
    let date = "2026-09-16".parse().unwrap();
    let mut first = resolution_input();
    first.issued_at = DeclaredProceduralTime::second(date, 12, 0, 0, Some(UtcOffset::UTC)).unwrap();
    let mut second = first.clone();
    second.issued_at =
        DeclaredProceduralTime::second(date, 6, 0, 0, Some(UtcOffset::from_hms(-6, 0, 0).unwrap()))
            .unwrap();
    assert_eq!(
        first.issued_at.instant_value(),
        second.issued_at.instant_value()
    );
    assert_ne!(
        ResolutionValues::new(first).canonical_bytes(),
        ResolutionValues::new(second).canonical_bytes()
    );
}

#[test]
fn normalized_text_is_canonical_but_unicode_composition_is_preserved() {
    let mut first = resolution_input();
    first.summary = text(" First\r\nSecond ");
    let mut second = first.clone();
    second.summary = text("First\nSecond");
    assert_eq!(
        ResolutionValues::new(first).canonical_bytes(),
        ResolutionValues::new(second).canonical_bytes()
    );
    let mut first = resolution_input();
    first.summary = text("\u{e1}");
    let mut second = first.clone();
    second.summary = text("a\u{301}");
    assert_ne!(
        ResolutionValues::new(first).canonical_bytes(),
        ResolutionValues::new(second).canonical_bytes()
    );
}

#[test]
fn notification_binding_and_function_locators_are_canonical_even_when_admission_is_shared() {
    let mut first = notification_input();
    first.provenance = external(Some(evidence(1, 1, 42, "Page 1")));
    first.representation = FactRepresentation::Declared {
        represented: person(1, 2),
        representative: person(2, 1),
        scope: text("Declared scope"),
        provenance: Box::new(external(Some(evidence(1, 1, 42, "Page 2")))),
    };
    let original = NotificationValues::new(first.clone()).unwrap();
    assert_eq!(original.direct_supports().len(), 1);
    assert!(original.canonical_bytes().starts_with(b"PFNOT1"));
    let mut changed = first.clone();
    changed.resolution.revision = FactRevision::new(2).unwrap();
    assert_ne!(
        original.canonical_bytes(),
        NotificationValues::new(changed).unwrap().canonical_bytes()
    );
    let mut changed = first.clone();
    let FactRepresentation::Declared { provenance, .. } = &mut changed.representation else {
        unreachable!()
    };
    **provenance = external(Some(evidence(1, 1, 42, "Page 3")));
    let changed = NotificationValues::new(changed).unwrap();
    assert_eq!(original.direct_supports(), changed.direct_supports());
    assert_ne!(original.canonical_bytes(), changed.canonical_bytes());
}

#[test]
fn optional_unknown_receipt_is_not_an_absent_receipt_or_stated_effect() {
    let first = notification_input();
    let original = NotificationValues::new(first.clone())
        .unwrap()
        .canonical_bytes();
    let mut receipt = first.clone();
    receipt.received_at = Some(DeclaredProceduralTime::unknown());
    let receipt = NotificationValues::new(receipt).unwrap().canonical_bytes();
    assert_ne!(original, receipt);
    let mut effect = first;
    effect.stated_effect = Some(FactStatedEffect {
        at: DeclaredProceduralTime::unknown(),
        statement: text("Declared effect"),
        locator: label("Paragraph 3"),
    });
    let effect = NotificationValues::new(effect).unwrap().canonical_bytes();
    assert_ne!(original, effect);
    assert_ne!(receipt, effect);
}
