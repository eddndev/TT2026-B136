use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::hearings::*;
use domain::participants::{ParticipantId, ParticipantRevision};
use serde_json::Value;
use time::{Date, Month, Time, UtcOffset};
use uuid::Uuid;

fn fixture_time(value: &str) -> HearingTime {
    let date = Date::from_calendar_date(
        value[0..4].parse().unwrap(),
        Month::try_from(value[5..7].parse::<u8>().unwrap()).unwrap(),
        value[8..10].parse().unwrap(),
    )
    .unwrap();
    let time = Time::from_hms(
        value[11..13].parse().unwrap(),
        value[14..16].parse().unwrap(),
        value[17..19].parse().unwrap(),
    )
    .unwrap();
    let sign = if &value[19..20] == "-" { -1 } else { 1 };
    let offset = UtcOffset::from_whole_seconds(
        sign * (value[20..22].parse::<i32>().unwrap() * 3600
            + value[23..25].parse::<i32>().unwrap() * 60),
    )
    .unwrap();
    HearingTime::new(date.with_time(time).assume_offset(offset)).unwrap()
}

fn fixture_input(value: &Value) -> HearingValuesInput {
    let basis = &value["conviction_basis"];
    HearingValuesInput {
        kind: value["kind"].as_str().unwrap().parse().unwrap(),
        scheduled_at: fixture_time(value["time"].as_str().unwrap()),
        modality: value["modality"].as_str().unwrap().parse().unwrap(),
        venue: HearingVenue::new(value["venue"].as_str().unwrap()).unwrap(),
        note: HearingNote::optional(value["note"].as_str()).unwrap(),
        participants: value["participants"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                HearingParticipantRef::new(
                    ParticipantId::from_uuid(
                        Uuid::parse_str(item["id"].as_str().unwrap()).unwrap(),
                    ),
                    ParticipantRevision::new(item["revision"].as_u64().unwrap() as u32).unwrap(),
                )
            })
            .collect(),
        conviction_basis: (!basis.is_null()).then(|| {
            HearingConvictionBasis::new(
                HearingNote::new(basis["statement"].as_str().unwrap()).unwrap(),
                HearingSupportRef::new(
                    DocumentVersionRef {
                        id: DocumentId::from_uuid(
                            Uuid::parse_str(basis["document_id"].as_str().unwrap()).unwrap(),
                        ),
                        version: DocumentVersion::new(basis["version"].as_u64().unwrap() as u32)
                            .unwrap(),
                    },
                    Sha256Digest::from_hex(basis["digest"].as_str().unwrap()).unwrap(),
                ),
            )
        }),
    }
}

fn rows() -> Vec<Value> {
    serde_json::from_str(include_str!("fixtures/hearing_vectors.json")).unwrap()
}

fn input() -> HearingValuesInput {
    fixture_input(&rows()[3]["input"])
}

fn canonical(input: HearingValuesInput) -> Vec<u8> {
    HearingValues::new(input).unwrap().canonical_bytes()
}

#[test]
fn all_kinds_match_independent_python_wire_vectors() {
    for row in rows() {
        let bytes = canonical(fixture_input(&row["input"]));
        let actual_hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            bytes.len(),
            row["bytes"].as_u64().unwrap() as usize,
            "{}",
            row["name"]
        );
        assert_eq!(actual_hex, row["hex"].as_str().unwrap(), "{}", row["name"]);
    }
}

#[test]
fn maximum_wire_size_includes_three_utf8_lengths_and_all_exact_references() {
    assert_eq!(MAX_HEARING_CANONICAL_BYTES, 10726);
    let row = rows().pop().unwrap();
    assert_eq!(
        canonical(fixture_input(&row["input"])).len(),
        MAX_HEARING_CANONICAL_BYTES
    );
}

#[test]
fn selection_permutations_and_normalized_text_have_the_same_encoding() {
    let first = input();
    let mut reordered = first.clone();
    reordered.participants.reverse();
    reordered.venue = HearingVenue::new(" Court A ").unwrap();
    reordered.note = Some(HearingNote::new(" \u{e1}\r\nb ").unwrap());
    assert_eq!(canonical(first), canonical(reordered));
}

#[test]
fn every_participant_permutation_has_the_same_encoding() {
    let mut base = input();
    base.participants = (1..=3)
        .map(|id| {
            HearingParticipantRef::new(
                ParticipantId::from_uuid(Uuid::from_u128(id)),
                ParticipantRevision::initial(),
            )
        })
        .collect();
    let expected = canonical(base.clone());
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut variant = base.clone();
        variant.participants = order
            .iter()
            .map(|index| base.participants[*index])
            .collect();
        assert_eq!(canonical(variant), expected);
    }
}

#[test]
fn each_declared_value_changes_canonical_bytes() {
    let base = input();
    let expected = canonical(base.clone());
    let mut variants = vec![];
    let mut value = base.clone();
    value.kind = HearingKind::Initial;
    value.conviction_basis = None;
    variants.push(value);
    let mut value = base.clone();
    value.scheduled_at = HearingTime::new(value.scheduled_at.utc()).unwrap();
    variants.push(value);
    let mut value = base.clone();
    value.scheduled_at = fixture_time("2026-01-01T00:10:01+14:00");
    variants.push(value);
    let mut value = base.clone();
    value.modality = HearingModality::Videoconference;
    variants.push(value);
    let mut value = base.clone();
    value.venue = HearingVenue::new("Court B").unwrap();
    variants.push(value);
    let mut value = base.clone();
    value.note = None;
    variants.push(value);
    let mut value = base.clone();
    value.note = Some(HearingNote::new("a\u{301}\nb").unwrap());
    variants.push(value);
    let mut value = base.clone();
    value.participants.clear();
    variants.push(value);
    let mut value = base.clone();
    let reference = value.participants[0];
    value.participants[0] =
        HearingParticipantRef::new(reference.id(), ParticipantRevision::new(8).unwrap());
    variants.push(value);
    for value in variants {
        assert_ne!(canonical(value), expected);
    }
}

#[test]
fn conviction_statement_document_version_and_digest_are_bound_independently() {
    let base = input();
    let expected = canonical(base.clone());
    let basis = base.conviction_basis.as_ref().unwrap();
    let proof = basis.support();
    let reference = proof.reference();
    for (statement, reference, digest) in [
        (
            HearingNote::new("Changed declaration").unwrap(),
            reference,
            proof.digest(),
        ),
        (
            basis.statement().clone(),
            DocumentVersionRef {
                id: DocumentId::from_uuid(Uuid::from_u128(7)),
                ..reference
            },
            proof.digest(),
        ),
        (
            basis.statement().clone(),
            DocumentVersionRef {
                version: DocumentVersion::new(8).unwrap(),
                ..reference
            },
            proof.digest(),
        ),
        (
            basis.statement().clone(),
            reference,
            Sha256Digest::from_array([0x99; 32]),
        ),
    ] {
        let mut value = base.clone();
        value.conviction_basis = Some(HearingConvictionBasis::new(
            statement,
            HearingSupportRef::new(reference, digest),
        ));
        assert_ne!(canonical(value), expected);
    }
}
