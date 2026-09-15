use super::{CaseStage, CaseStageChange, DeclaredStageTime, StageSupportRef, StageTransition};

impl CaseStageChange {
    /// Encodes normalized declared values, excluding context and captured provenance.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"CSTG1".to_vec();
        match self {
            Self::Adopt(values) => {
                bytes.push(0);
                bytes.push(match values.stage() {
                    CaseStage::Investigation => 0,
                    CaseStage::Intermediate => 1,
                    CaseStage::Trial => 2,
                });
                declared_time(&mut bytes, values.known_at());
                text(&mut bytes, values.reason().as_str());
                support(&mut bytes, values.support());
            }
            Self::Transition(StageTransition::ToIntermediate(values)) => {
                bytes.push(1);
                declared_time(&mut bytes, values.accusation_declared_at());
                support(&mut bytes, values.accusation());
                optional(&mut bytes, values.note(), |bytes, note| {
                    text(bytes, note.as_str())
                });
            }
            Self::Transition(StageTransition::ToTrial(values)) => {
                bytes.push(2);
                declared_time(&mut bytes, values.opening_order_issued_at());
                support(&mut bytes, values.opening_order());
                declared_time(&mut bytes, values.received_at());
                text(&mut bytes, values.receiving_court().as_str());
                optional(
                    &mut bytes,
                    values.receipt_reference(),
                    |bytes, reference| {
                        text(bytes, reference.as_str());
                    },
                );
                optional(&mut bytes, values.receipt_support(), support);
                optional(&mut bytes, values.note(), |bytes, note| {
                    text(bytes, note.as_str())
                });
            }
        }
        bytes
    }
}

fn declared_time(bytes: &mut Vec<u8>, value: DeclaredStageTime) {
    match value.instant_value() {
        None => {
            bytes.push(0);
            bytes.extend_from_slice(&(value.local_date().year() as u16).to_be_bytes());
            bytes.push(value.local_date().month() as u8);
            bytes.push(value.local_date().day());
        }
        Some(instant) => {
            bytes.push(1);
            bytes.extend_from_slice(&instant.unix_timestamp().to_be_bytes());
            bytes.extend_from_slice(&instant.nanosecond().to_be_bytes());
        }
    }
    bytes.extend_from_slice(&value.offset().whole_seconds().to_be_bytes());
}

fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn support(bytes: &mut Vec<u8>, value: StageSupportRef) {
    bytes.extend_from_slice(value.reference().id.as_uuid().as_bytes());
    bytes.extend_from_slice(&value.reference().version.get().to_be_bytes());
    bytes.extend_from_slice(value.digest().as_bytes());
}

fn optional<T>(bytes: &mut Vec<u8>, value: Option<T>, encode: impl FnOnce(&mut Vec<u8>, T)) {
    bytes.push(u8::from(value.is_some()));
    if let Some(value) = value {
        encode(bytes, value);
    }
}
