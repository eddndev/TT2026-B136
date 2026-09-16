#![allow(dead_code)]
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::hearing_results::*;
use domain::participants::{ParticipantId, ParticipantRevision};
use serde_json::Value;
use time::{Date, Month, Time, UtcOffset};
use uuid::Uuid;

pub fn attendee(id: u128, revision: u32) -> HearingResultAttendee {
    HearingResultAttendee::new(
        ParticipantId::from_uuid(Uuid::from_u128(id)),
        ParticipantRevision::new(revision).unwrap(),
        HearingResultCapacity::new(" Counsel ").unwrap(),
        None,
    )
}
pub fn agreement(id: u128, text: &str) -> HearingResultAgreement {
    HearingResultAgreement::new(
        HearingResultAgreementId::from_uuid(Uuid::from_u128(id)),
        HearingResultText::new(text).unwrap(),
    )
}
pub fn support() -> HearingResultSupportRef {
    HearingResultSupportRef::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(77)),
            version: DocumentVersion::new(4).unwrap(),
        },
        Sha256Digest::from_array([0x42; 32]),
    )
}
pub fn input() -> HearingResultValuesInput {
    HearingResultValuesInput {
        occurrence: HearingResultOccurrence::Occurred,
        extent: HearingResultExtent::Unspecified,
        event_time: DeclaredHearingResultTime::date(
            Date::from_calendar_date(2026, Month::September, 16).unwrap(),
            UtcOffset::UTC,
        )
        .unwrap(),
        summary: HearingResultText::new(" Declared result ").unwrap(),
        attendees: vec![],
        agreements: vec![],
        provenance: HearingResultProvenance::new(
            HearingResultProvenanceKind::OperatorNote,
            None,
            None,
        )
        .unwrap(),
    }
}
pub fn fixture_time(value: &Value) -> DeclaredHearingResultTime {
    let stamp = value["at"].as_str().unwrap_or("");
    let date = value["date"].as_str().unwrap_or(stamp);
    let day = Date::from_calendar_date(
        date[0..4].parse().unwrap(),
        Month::try_from(date[5..7].parse::<u8>().unwrap()).unwrap(),
        date[8..10].parse().unwrap(),
    )
    .unwrap();
    let offset = value["offset"].as_str().unwrap_or_else(|| &stamp[19..]);
    let sign = if offset.starts_with('-') { -1 } else { 1 };
    let offset = UtcOffset::from_whole_seconds(
        sign * (offset[1..3].parse::<i32>().unwrap() * 3600
            + offset[4..6].parse::<i32>().unwrap() * 60),
    )
    .unwrap();
    if value["precision"] == "date" {
        DeclaredHearingResultTime::date(day, offset).unwrap()
    } else {
        let time = Time::from_hms(
            stamp[11..13].parse().unwrap(),
            stamp[14..16].parse().unwrap(),
            stamp[17..19].parse().unwrap(),
        )
        .unwrap();
        DeclaredHearingResultTime::instant(day.with_time(time).assume_offset(offset)).unwrap()
    }
}
pub fn fixture_input(value: &Value) -> HearingResultValuesInput {
    let source = &value["provenance"];
    HearingResultValuesInput {
        occurrence: value["occurrence"].as_str().unwrap().parse().unwrap(),
        extent: value["extent"].as_str().unwrap().parse().unwrap(),
        event_time: fixture_time(&value["event_time"]),
        summary: HearingResultText::new(value["summary"].as_str().unwrap()).unwrap(),
        attendees: value["attendees"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                HearingResultAttendee::new(
                    ParticipantId::from_uuid(
                        Uuid::parse_str(item["participant_id"].as_str().unwrap()).unwrap(),
                    ),
                    ParticipantRevision::new(item["revision"].as_u64().unwrap() as u32).unwrap(),
                    HearingResultCapacity::new(item["capacity"].as_str().unwrap()).unwrap(),
                    HearingResultObservation::optional(item["observation"].as_str()).unwrap(),
                )
            })
            .collect(),
        agreements: value["agreements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                HearingResultAgreement::new(
                    HearingResultAgreementId::from_uuid(
                        Uuid::parse_str(item["id"].as_str().unwrap()).unwrap(),
                    ),
                    HearingResultText::new(item["text"].as_str().unwrap()).unwrap(),
                )
            })
            .collect(),
        provenance: HearingResultProvenance::new(
            source["kind"].as_str().unwrap().parse().unwrap(),
            HearingResultReference::optional(source["reference"].as_str()).unwrap(),
            (!source["support"].is_null()).then(|| {
                let proof = &source["support"];
                HearingResultSupportRef::new(
                    DocumentVersionRef {
                        id: DocumentId::from_uuid(
                            Uuid::parse_str(proof["document_id"].as_str().unwrap()).unwrap(),
                        ),
                        version: DocumentVersion::new(proof["version"].as_u64().unwrap() as u32)
                            .unwrap(),
                    },
                    Sha256Digest::from_hex(proof["digest"].as_str().unwrap()).unwrap(),
                )
            }),
        )
        .unwrap(),
    }
}
pub fn rows() -> Vec<Value> {
    serde_json::from_str(include_str!("../fixtures/hearing_result_vectors.json")).unwrap()
}
pub fn canonical(input: HearingResultValuesInput) -> Vec<u8> {
    HearingResultValues::new(input).unwrap().canonical_bytes()
}
