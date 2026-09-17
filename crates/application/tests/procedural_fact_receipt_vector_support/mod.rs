mod sources;
pub use sources::sources;

use application::procedural_facts::*;
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    judicial_calendars::CivilDate,
    procedural_time::DeclaredProceduralTime,
    DomainError,
};
use serde_json::Value;
use std::{cell::Cell, io::Read};
use time::UtcOffset;
use uuid::Uuid;

pub fn fixtures() -> Value {
    serde_json::from_str(include_str!("../fixtures/procedural-fact-receipts.json")).unwrap()
}
pub fn string(value: &Value) -> &str {
    value.as_str().expect("fixture string")
}
pub fn number(value: &Value) -> u32 {
    value.as_u64().unwrap().try_into().unwrap()
}
pub fn byte(value: &Value) -> u8 {
    number(value).try_into().unwrap()
}
pub fn uuid(value: &Value) -> Uuid {
    Uuid::parse_str(string(value)).unwrap()
}
pub fn bytes(value: &Value) -> Vec<u8> {
    let value = string(value);
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
pub fn digest(value: &Value) -> Sha256Digest {
    Sha256Digest::from_array(bytes(value).try_into().unwrap())
}
pub fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub fn offset(value: &Value) -> UtcOffset {
    UtcOffset::from_whole_seconds(value.as_i64().unwrap().try_into().unwrap()).unwrap()
}
pub fn declared_time(value: &Value) -> DeclaredProceduralTime {
    if value["precision"] == "unknown" {
        return DeclaredProceduralTime::unknown();
    }
    let date: CivilDate = string(&value["date"]).parse().unwrap();
    let offset = (!value["offset_seconds"].is_null()).then(|| offset(&value["offset_seconds"]));
    match string(&value["precision"]) {
        "date" => DeclaredProceduralTime::date(date, offset),
        "minute" => DeclaredProceduralTime::minute(
            date,
            byte(&value["hour"]),
            byte(&value["minute"]),
            offset,
        ),
        "second" => DeclaredProceduralTime::second(
            date,
            byte(&value["hour"]),
            byte(&value["minute"]),
            byte(&value["second"]),
            offset,
        ),
        _ => panic!("unsupported fixture precision"),
    }
    .unwrap()
}

/// Verifies the exact bytes passed to the port; SHA-256 is supplied by Python.
pub struct OracleHasher {
    expected: Vec<u8>,
    output: Sha256Digest,
    calls: Cell<usize>,
}
impl OracleHasher {
    pub fn new(vector: &Value) -> Self {
        Self {
            expected: bytes(&vector["hex"]),
            output: digest(&vector["sha256"]),
            calls: Cell::new(0),
        }
    }
    pub fn assert_used_once(&self) {
        assert_eq!(self.calls.get(), 1);
    }
}
impl DocumentHasher for OracleHasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        assert_eq!(data, self.expected);
        self.calls.set(self.calls.get() + 1);
        self.output
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("canonical receipts use the bounded bytes port")
    }
}

fn resolution(summary: &str) -> ResolutionValues {
    ResolutionValues::new(ResolutionValuesInput {
        class: FactDeclaration::Known(ResolutionClass::Order),
        subtype: None,
        issuer: FactDeclaration::Unknown(text("Unknown issuer")),
        issued_at: DeclaredProceduralTime::unknown(),
        summary: text(summary),
        provenance: FactProvenance::OperatorNote { note: text("Note") },
    })
}
fn notification(parent: ResolutionId, summary: &str) -> NotificationValues {
    NotificationValues::new(NotificationValuesInput {
        resolution: FactResolutionRef {
            id: parent,
            revision: FactRevision::initial(),
        },
        character: FactDeclaration::Unknown(text("Unknown character")),
        medium: FactDeclaration::Unknown(text("Unknown medium")),
        context: FactDeclaration::Unknown(text("Unknown context")),
        outcome: FactDeclaration::Unknown(text("Unknown outcome")),
        subtype: None,
        practiced_at: DeclaredProceduralTime::unknown(),
        received_at: None,
        stated_effect: None,
        intended_recipient: FactDeclaration::Unknown(text("Unknown recipient")),
        actual_receiver: FactDeclaration::Unknown(text("Unknown receiver")),
        representation: FactRepresentation::NotRecorded(text("Not recorded")),
        summary: text(summary),
        provenance: FactProvenance::OperatorNote { note: text("Note") },
    })
    .unwrap()
}
fn change<T>(value: &Value, values: T) -> FactChange<T> {
    match string(&value["action"]) {
        "record" => FactChange::record(values),
        "correct" => FactChange::correct(
            FactRevision::new(number(&value["expected_revision"])).unwrap(),
            values,
            text(string(&value["reason"])),
        ),
        "withdraw" => FactChange::withdraw(
            FactRevision::new(number(&value["expected_revision"])).unwrap(),
            text(string(&value["reason"])),
        ),
        _ => panic!("unsupported fixture action"),
    }
}
pub fn command(value: &Value, summary: &str) -> ProceduralFactCommand {
    let operation = FactOperationId::from_uuid(uuid(&value["operation_id"]));
    match string(&value["family"]) {
        "resolution" => ProceduralFactCommand::Resolution(ResolutionCommand::new(
            operation,
            ResolutionId::from_uuid(uuid(&value["target_id"])),
            change(value, resolution(summary)),
        )),
        "notification" => {
            let parent = ResolutionId::from_uuid(uuid(&value["parent_id"]));
            ProceduralFactCommand::Notification(
                NotificationCommand::new(
                    operation,
                    NotificationId::from_uuid(uuid(&value["target_id"])),
                    parent,
                    change(value, notification(parent, summary)),
                )
                .unwrap(),
            )
        }
        _ => panic!("unsupported fixture family"),
    }
}
