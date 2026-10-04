use super::{MeasureDecisionOutcome, MeasureEffect, MeasureProposal};
use crate::precautionary_hearings::PrecautionaryMeasureRef;

impl MeasureDecisionOutcome {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"MEFX1".to_vec();
        if let Some(effects) = self.changes() {
            bytes.push(0);
            bytes.extend_from_slice(&(effects.len() as u32).to_be_bytes());
            for effect in effects {
                encode_effect(&mut bytes, effect);
            }
        } else if let Some(statement) = self.no_measure_change() {
            bytes.push(1);
            blob(&mut bytes, statement.as_str().as_bytes());
        }
        bytes
    }
}

fn encode_effect(bytes: &mut Vec<u8>, effect: &MeasureEffect) {
    match effect {
        MeasureEffect::Impose(value) => {
            bytes.push(0);
            proposal(bytes, value);
        }
        MeasureEffect::Confirm { previous } => {
            bytes.push(1);
            reference(bytes, *previous);
        }
        MeasureEffect::Modify { previous, values } => {
            bytes.push(2);
            reference(bytes, *previous);
            blob(bytes, &values.canonical_bytes());
        }
        MeasureEffect::Revoke { previous } => {
            bytes.push(3);
            reference(bytes, *previous);
        }
        MeasureEffect::Cease { previous } => {
            bytes.push(4);
            reference(bytes, *previous);
        }
        MeasureEffect::Substitute {
            predecessors,
            successors,
        } => {
            bytes.push(5);
            bytes.extend_from_slice(&(predecessors.len() as u32).to_be_bytes());
            for item in predecessors {
                reference(bytes, *item);
            }
            bytes.extend_from_slice(&(successors.len() as u32).to_be_bytes());
            for item in successors {
                proposal(bytes, item);
            }
        }
    }
}

fn proposal(bytes: &mut Vec<u8>, value: &MeasureProposal) {
    bytes.extend_from_slice(value.id.as_uuid().as_bytes());
    blob(bytes, &value.values.canonical_bytes());
}

fn reference(bytes: &mut Vec<u8>, value: PrecautionaryMeasureRef) {
    bytes.extend_from_slice(value.id().as_uuid().as_bytes());
    bytes.extend_from_slice(&value.revision().get().to_be_bytes());
    bytes.extend_from_slice(value.digest().as_bytes());
}

fn blob(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value);
}
