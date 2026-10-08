use super::{MeasureSupervision, MeasureValidity, MeasureValues, MeasureValuesInput};
use crate::{hearings::HearingNote, DomainError};

/// Permitted transcription edits; applicability requires an exact prior record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureCorrectionValues {
    conditions: HearingNote,
    validity: MeasureValidity,
    supervision_text: HearingNote,
}

impl MeasureCorrectionValues {
    pub fn new(
        conditions: HearingNote,
        validity: MeasureValidity,
        supervision_text: HearingNote,
    ) -> Self {
        Self {
            conditions,
            validity,
            supervision_text,
        }
    }

    pub const fn conditions(&self) -> &HearingNote {
        &self.conditions
    }
    pub const fn validity(&self) -> &MeasureValidity {
        &self.validity
    }
    pub const fn supervision_text(&self) -> &HearingNote {
        &self.supervision_text
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"MCVAL1".to_vec();
        super::canonical::text(&mut bytes, self.conditions.as_str());
        let validity = self.validity.canonical_bytes();
        bytes.extend_from_slice(&(validity.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&validity);
        super::canonical::text(&mut bytes, self.supervision_text.as_str());
        bytes
    }
}

impl MeasureValues {
    /// Preserve identity-bearing fields and existing optional/variant structure.
    /// This does not establish faithful transcription, authorization or a new effect.
    pub fn correct_record(
        &self,
        correction: &MeasureCorrectionValues,
    ) -> Result<Self, DomainError> {
        if self.validity().end().is_some() != correction.validity.end().is_some() {
            return Err(DomainError::InvalidPrecautionaryMeasure(
                "correction_end_presence",
            ));
        }
        let supervision = match self.supervision() {
            MeasureSupervision::Known { participant, .. } => MeasureSupervision::Known {
                participant: *participant,
                statement: correction.supervision_text.clone(),
            },
            MeasureSupervision::Unknown { .. } => MeasureSupervision::Unknown {
                reason: correction.supervision_text.clone(),
            },
        };
        let result = Self::new(MeasureValuesInput {
            subject: self.subject(),
            kind: self.kind(),
            conditions: correction.conditions.clone(),
            validity: correction.validity.clone(),
            supervision,
        });
        if result == *self {
            return Err(DomainError::InvalidPrecautionaryMeasure(
                "correction_unchanged",
            ));
        }
        Ok(result)
    }
}
