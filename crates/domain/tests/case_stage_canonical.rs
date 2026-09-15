use domain::case_stages::{
    CaseStage, CaseStageChange, DeclaredStageTime, StageAdoption, StageCourt, StageNote,
    StageReceiptReference, StageSupportRef, StageTransition,
};
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use time::macros::{date, datetime, offset};

fn support() -> StageSupportRef {
    StageSupportRef::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(uuid::Uuid::from_u128(0x00112233445566778899aabbccddeeff)),
            version: DocumentVersion::new(7).unwrap(),
        },
        Sha256Digest::from_array(std::array::from_fn(|i| i as u8)),
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn instant() -> DeclaredStageTime {
    DeclaredStageTime::instant(datetime!(2026-09-14 21:00:00.123_456_789 -6)).unwrap()
}

#[test]
fn adoption_day_matches_independently_encoded_vector() {
    let change = CaseStageChange::Adopt(StageAdoption::new(
        CaseStage::Trial,
        DeclaredStageTime::date(date!(2026 - 09 - 14), offset!(-6)).unwrap(),
        StageNote::new("Legado\r\nVerificado").unwrap(),
        support(),
    ));
    assert_eq!(change.canonical_bytes().len(), 89);
    assert_eq!(
        hex(&change.canonical_bytes()),
        concat!(
            "435354473100020007ea090effffaba0000000114c656761646f0a5665726966696361646f",
            "00112233445566778899aabbccddeeff00000007",
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
        )
    );
}

#[test]
fn intermediate_instant_matches_independently_encoded_vector() {
    let change =
        CaseStageChange::Transition(StageTransition::to_intermediate(instant(), support(), None));
    assert_eq!(change.canonical_bytes().len(), 76);
    assert_eq!(
        hex(&change.canonical_bytes()),
        concat!(
            "43535447310101000000006aa8b4b0075bcd15ffffaba0",
            "00112233445566778899aabbccddeeff00000007",
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f00"
        )
    );
}

#[test]
fn mixed_trial_vector_keeps_both_support_roles() {
    let change = CaseStageChange::Transition(
        StageTransition::to_trial(
            DeclaredStageTime::date(date!(2026 - 09 - 14), offset!(-6)).unwrap(),
            support(),
            DeclaredStageTime::instant(datetime!(2026-09-15 00:00:01.000_000_001 +5:30)).unwrap(),
            StageCourt::new("Juzgado \u{e1}").unwrap(),
            Some(StageReceiptReference::new("R-1").unwrap()),
            Some(support()),
            Some(StageNote::new("\u{e1}\nb").unwrap()),
        )
        .unwrap(),
    );
    assert_eq!(change.supports(), vec![support()]);
    assert_eq!(change.canonical_bytes().len(), 168);
    assert_eq!(
        hex(&change.canonical_bytes()),
        concat!(
            "4353544731020007ea090effffaba0",
            "00112233445566778899aabbccddeeff00000007",
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
            "01000000006aa83d290000000100004d580000000a4a757a6761646f20c3a1",
            "0100000003522d3101",
            "00112233445566778899aabbccddeeff00000007",
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
            "0100000004c3a10a62"
        )
    );
}

#[test]
fn canonical_maximum_counts_utf8_bytes_of_all_bounded_fields() {
    let note = StageNote::new(&"\u{1f4dc}".repeat(1000)).unwrap();
    let short = "\u{1f4dc}".repeat(200);
    let adoption = CaseStageChange::Adopt(StageAdoption::new(
        CaseStage::Investigation,
        instant(),
        note.clone(),
        support(),
    ));
    assert_eq!(adoption.canonical_bytes().len(), 4080);
    let intermediate = CaseStageChange::Transition(StageTransition::to_intermediate(
        instant(),
        support(),
        Some(note.clone()),
    ));
    assert_eq!(intermediate.canonical_bytes().len(), 4080);
    let trial = CaseStageChange::Transition(
        StageTransition::to_trial(
            instant(),
            support(),
            instant(),
            StageCourt::new(&short).unwrap(),
            Some(StageReceiptReference::new(&short).unwrap()),
            Some(support()),
            Some(note),
        )
        .unwrap(),
    );
    assert_eq!(trial.canonical_bytes().len(), 5759);
}

#[test]
fn identical_instants_with_distinct_declared_offsets_are_distinct_values() {
    let first = DeclaredStageTime::instant(datetime!(2026-09-15 00:00 -6)).unwrap();
    let second = DeclaredStageTime::instant(datetime!(2026-09-15 06:00 UTC)).unwrap();
    assert_eq!(first.lower_bound(), second.lower_bound());
    assert_ne!(first, second);
    let encode = |time| {
        CaseStageChange::Transition(StageTransition::to_intermediate(time, support(), None))
            .canonical_bytes()
    };
    assert_ne!(encode(first), encode(second));
    let day = DeclaredStageTime::date(date!(2026 - 09 - 15), offset!(-6)).unwrap();
    assert_ne!(encode(first), encode(day));
}

#[test]
fn optional_fields_and_exact_versions_change_the_canonical_values() {
    let base = |reference, receipt, note| {
        CaseStageChange::Transition(
            StageTransition::to_trial(
                instant(),
                support(),
                instant(),
                StageCourt::new("Court").unwrap(),
                reference,
                receipt,
                note,
            )
            .unwrap(),
        )
        .canonical_bytes()
    };
    let none = base(None, None, None);
    assert_ne!(
        none,
        base(Some(StageReceiptReference::new("R").unwrap()), None, None)
    );
    assert_ne!(none, base(None, Some(support()), None));
    assert_ne!(none, base(None, None, Some(StageNote::new("N").unwrap())));
    let newer = StageSupportRef::new(
        DocumentVersionRef {
            version: DocumentVersion::new(8).unwrap(),
            ..support().reference()
        },
        support().digest(),
    );
    assert_ne!(
        base(None, Some(support()), None),
        base(None, Some(newer), None)
    );
}

#[test]
fn canonical_text_uses_normalized_values_without_folding_unicode() {
    let encode = |note: &str| {
        CaseStageChange::Transition(StageTransition::to_intermediate(
            instant(),
            support(),
            Some(StageNote::new(note).unwrap()),
        ))
        .canonical_bytes()
    };
    assert_eq!(encode(" \u{e1}\r\nb "), encode("\u{e1}\nb"));
    assert_ne!(encode("\u{e1}"), encode("a\u{301}"));
}
