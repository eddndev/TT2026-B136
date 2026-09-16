use super::*;
use crate::procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime};

pub(super) fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
pub(super) fn optional<T>(
    bytes: &mut Vec<u8>,
    value: Option<&T>,
    encode: impl FnOnce(&mut Vec<u8>, &T),
) {
    bytes.push(u8::from(value.is_some()));
    if let Some(value) = value {
        encode(bytes, value);
    }
}
pub(super) fn declaration<T>(
    bytes: &mut Vec<u8>,
    value: &FactDeclaration<T>,
    encode: impl FnOnce(&mut Vec<u8>, &T),
) {
    match value {
        FactDeclaration::Unknown(reason) => {
            bytes.push(0);
            text(bytes, reason.as_str());
        }
        FactDeclaration::Known(value) => {
            bytes.push(1);
            encode(bytes, value);
        }
    }
}
pub(super) fn declared_time(bytes: &mut Vec<u8>, value: DeclaredProceduralTime) {
    bytes.push(match value.precision() {
        DeclaredProceduralPrecision::Unknown => 0,
        DeclaredProceduralPrecision::Date => 1,
        DeclaredProceduralPrecision::Minute => 2,
        DeclaredProceduralPrecision::Second => 3,
    });
    if let Some(date) = value.local_date() {
        let date = date.date();
        bytes.extend_from_slice(&(date.year() as u16).to_be_bytes());
        bytes.push(date.month() as u8);
        bytes.push(date.day());
        if let Some(hour) = value.local_hour() {
            bytes.push(hour);
        }
        if let Some(minute) = value.local_minute() {
            bytes.push(minute);
        }
        if let Some(second) = value.local_second() {
            bytes.push(second);
        }
        optional(bytes, value.offset().as_ref(), |b, offset| {
            b.extend_from_slice(&offset.whole_seconds().to_be_bytes());
        });
    }
}
pub(super) fn person(bytes: &mut Vec<u8>, value: &FactPerson) {
    match value {
        FactPerson::Participant(reference) => {
            bytes.push(0);
            bytes.extend_from_slice(reference.id.as_uuid().as_bytes());
            bytes.extend_from_slice(&reference.revision.get().to_be_bytes());
        }
        FactPerson::Unlinked { label, description } => {
            bytes.push(1);
            text(bytes, label.as_str());
            text(bytes, description.as_str());
        }
    }
}
pub(super) fn representation(bytes: &mut Vec<u8>, value: &FactRepresentation) {
    match value {
        FactRepresentation::NotRecorded(reason) => {
            bytes.push(0);
            text(bytes, reason.as_str());
        }
        FactRepresentation::Declared {
            represented,
            representative,
            scope,
            provenance: source,
        } => {
            bytes.push(1);
            person(bytes, represented);
            person(bytes, representative);
            text(bytes, scope.as_str());
            provenance(bytes, source);
        }
    }
}
pub(super) fn provenance(bytes: &mut Vec<u8>, value: &FactProvenance) {
    match value {
        FactProvenance::OperatorNote { note } => {
            bytes.push(0);
            text(bytes, note.as_str());
        }
        FactProvenance::ExternalReference { reference, support } => {
            bytes.push(1);
            text(bytes, reference.as_str());
            optional(bytes, support.as_ref(), evidence);
        }
        FactProvenance::HearingResult {
            reference,
            locator,
            support,
        } => {
            bytes.push(2);
            bytes.extend_from_slice(reference.hearing_id.as_uuid().as_bytes());
            bytes.extend_from_slice(reference.result_id.as_uuid().as_bytes());
            bytes.extend_from_slice(&reference.revision.get().to_be_bytes());
            optional(bytes, reference.agreement_id.as_ref(), |b, agreement| {
                b.extend_from_slice(agreement.as_uuid().as_bytes());
            });
            text(bytes, locator.as_str());
            optional(bytes, support.as_ref(), evidence);
        }
    }
}
fn evidence(bytes: &mut Vec<u8>, value: &FactEvidence) {
    bytes.extend_from_slice(value.reference().id.as_uuid().as_bytes());
    bytes.extend_from_slice(&value.reference().version.get().to_be_bytes());
    bytes.extend_from_slice(value.digest().as_bytes());
    text(bytes, value.locator().as_str());
}
