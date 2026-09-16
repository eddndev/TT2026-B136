use super::{label, text};
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    hearing_results::{HearingResultAgreementId, HearingResultId, HearingResultRevision},
    hearings::HearingId,
    judicial_calendars::CivilDate,
    participants::{ParticipantId, ParticipantRevision},
    procedural_facts::*,
    procedural_time::DeclaredProceduralTime,
};
use serde_json::Value;
use time::{Date, Month, UtcOffset};
use uuid::Uuid;

fn s(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap().to_owned()
}
fn id(value: &Value, key: &str) -> Uuid {
    Uuid::parse_str(&s(value, key)).unwrap()
}
fn n(value: &Value, key: &str) -> u32 {
    value[key].as_u64().unwrap().try_into().unwrap()
}
fn opt<T>(value: &Value, f: impl FnOnce(&Value) -> T) -> Option<T> {
    (!value.is_null()).then(|| f(value))
}
fn declaration<T>(value: &Value, f: impl FnOnce(&Value) -> T) -> FactDeclaration<T> {
    match s(value, "kind").as_str() {
        "unknown" => FactDeclaration::Unknown(text(&s(value, "reason"))),
        "known" => FactDeclaration::Known(f(&value["value"])),
        _ => panic!("invalid fixture declaration"),
    }
}
fn time(value: &Value) -> DeclaredProceduralTime {
    let precision = s(value, "precision");
    if precision == "unknown" {
        return DeclaredProceduralTime::unknown();
    }
    let date = CivilDate::from_date(
        Date::from_calendar_date(
            n(value, "year") as i32,
            Month::try_from(n(value, "month") as u8).unwrap(),
            n(value, "day") as u8,
        )
        .unwrap(),
    )
    .unwrap();
    let offset = opt(&value["offset_seconds"], |v| {
        UtcOffset::from_whole_seconds(v.as_i64().unwrap() as i32).unwrap()
    });
    match precision.as_str() {
        "date" => DeclaredProceduralTime::date(date, offset),
        "minute" => DeclaredProceduralTime::minute(
            date,
            n(value, "hour") as u8,
            n(value, "minute") as u8,
            offset,
        ),
        "second" => DeclaredProceduralTime::second(
            date,
            n(value, "hour") as u8,
            n(value, "minute") as u8,
            n(value, "second") as u8,
            offset,
        ),
        _ => panic!("invalid fixture time"),
    }
    .unwrap()
}
fn person(value: &Value) -> FactPerson {
    match s(value, "kind").as_str() {
        "participant" => FactPerson::Participant(FactParticipantRef {
            id: ParticipantId::from_uuid(id(value, "id")),
            revision: ParticipantRevision::new(n(value, "revision")).unwrap(),
        }),
        "unlinked" => FactPerson::Unlinked {
            label: label(&s(value, "label")),
            description: text(&s(value, "description")),
        },
        _ => panic!("invalid fixture person"),
    }
}
fn evidence(value: &Value) -> FactEvidence {
    FactEvidence::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(id(value, "document_id")),
            version: DocumentVersion::new(n(value, "version")).unwrap(),
        },
        Sha256Digest::from_hex(&s(value, "digest")).unwrap(),
        label(&s(value, "locator")),
    )
}
fn provenance(value: &Value) -> FactProvenance {
    match s(value, "kind").as_str() {
        "operator_note" => FactProvenance::OperatorNote {
            note: text(&s(value, "note")),
        },
        "external_reference" => FactProvenance::ExternalReference {
            reference: text(&s(value, "reference")),
            support: opt(&value["support"], evidence),
        },
        "hearing_result" => {
            let r = &value["reference"];
            FactProvenance::HearingResult {
                reference: FactHearingRef {
                    hearing_id: HearingId::from_uuid(id(r, "hearing_id")),
                    result_id: HearingResultId::from_uuid(id(r, "result_id")),
                    revision: HearingResultRevision::new(n(r, "revision")).unwrap(),
                    agreement_id: opt(&r["agreement_id"], |v| {
                        HearingResultAgreementId::from_uuid(
                            Uuid::parse_str(v.as_str().unwrap()).unwrap(),
                        )
                    }),
                },
                locator: label(&s(value, "locator")),
                support: opt(&value["support"], evidence),
            }
        }
        _ => panic!("invalid fixture provenance"),
    }
}
fn representation(value: &Value) -> FactRepresentation {
    match s(value, "kind").as_str() {
        "not_recorded" => FactRepresentation::NotRecorded(text(&s(value, "reason"))),
        "declared" => FactRepresentation::Declared {
            represented: person(&value["represented"]),
            representative: person(&value["representative"]),
            scope: text(&s(value, "scope")),
            provenance: Box::new(provenance(&value["provenance"])),
        },
        _ => panic!("invalid fixture representation"),
    }
}
fn class(value: &Value) -> ResolutionClass {
    match s(value, "kind").as_str() {
        "order" => ResolutionClass::Order,
        "judgment" => ResolutionClass::Judgment,
        "other" => ResolutionClass::Other(label(&s(value, "label"))),
        _ => panic!("invalid fixture class"),
    }
}
pub fn resolution(value: &Value) -> ResolutionValues {
    ResolutionValues::new(ResolutionValuesInput {
        class: declaration(&value["class"], class),
        subtype: opt(&value["subtype"], |v| label(v.as_str().unwrap())),
        issuer: declaration(&value["issuer"], |v| label(v.as_str().unwrap())),
        issued_at: time(&value["issued_at"]),
        summary: text(&s(value, "summary")),
        provenance: provenance(&value["provenance"]),
    })
}
pub fn notification(value: &Value) -> NotificationValues {
    NotificationValues::new(NotificationValuesInput {
        resolution: FactResolutionRef {
            id: ResolutionId::from_uuid(id(&value["resolution"], "id")),
            revision: FactRevision::new(n(&value["resolution"], "revision")).unwrap(),
        },
        character: declaration(&value["character"], |v| match s(v, "kind").as_str() {
            "personal" => NotificationCharacter::Personal,
            "publication" => NotificationCharacter::Publication,
            "other" => NotificationCharacter::Other(label(&s(v, "label"))),
            _ => panic!("invalid character"),
        }),
        medium: declaration(&value["medium"], |v| match s(v, "kind").as_str() {
            "in_person" => NotificationMedium::InPerson,
            "electronic" => NotificationMedium::Electronic,
            "other" => NotificationMedium::Other(label(&s(v, "label"))),
            _ => panic!("invalid medium"),
        }),
        context: declaration(&value["context"], |v| match s(v, "kind").as_str() {
            "in_hearing" => NotificationContext::InHearing,
            "outside_hearing" => NotificationContext::OutsideHearing,
            "other" => NotificationContext::Other(label(&s(v, "label"))),
            _ => panic!("invalid context"),
        }),
        outcome: declaration(&value["outcome"], |v| match s(v, "kind").as_str() {
            "practiced" => NotificationOutcome::Practiced,
            "attempted" => NotificationOutcome::Attempted,
            _ => panic!("invalid outcome"),
        }),
        subtype: opt(&value["subtype"], |v| label(v.as_str().unwrap())),
        practiced_at: time(&value["practiced_at"]),
        received_at: opt(&value["received_at"], time),
        stated_effect: opt(&value["stated_effect"], |v| FactStatedEffect {
            at: time(&v["at"]),
            statement: text(&s(v, "statement")),
            locator: label(&s(v, "locator")),
        }),
        intended_recipient: declaration(&value["intended_recipient"], person),
        actual_receiver: declaration(&value["actual_receiver"], person),
        representation: representation(&value["representation"]),
        summary: text(&s(value, "summary")),
        provenance: provenance(&value["provenance"]),
    })
    .unwrap()
}
