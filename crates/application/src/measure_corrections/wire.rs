use super::*;
use crate::{
    precautionary_measures::{decision_wire, MeasureCaptureAction},
    ApplicationError,
};
pub(super) use decision_wire::{actor, blob, reference, support, timestamp};

pub(super) fn invalid(message: &str) -> ApplicationError {
    ApplicationError::InvalidInput(format!(
        "inconsistent administrative measure record: {message}"
    ))
}

pub(super) fn result(
    bytes: &mut Vec<u8>,
    value: &MeasureAdministrativeResult,
) -> Result<(), ApplicationError> {
    bytes.extend_from_slice(value.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&value.revision.get().to_be_bytes());
    reference(bytes, value.previous);
    match &value.record_root {
        MeasureRecordRoot::Judicial(origin) => {
            bytes.push(0);
            bytes.extend_from_slice(origin.decision_id.as_uuid().as_bytes());
            bytes.extend_from_slice(origin.operation_id.as_uuid().as_bytes());
        }
        MeasureRecordRoot::Administrative {
            operation_id,
            measure_id,
        } => {
            bytes.push(1);
            bytes.extend_from_slice(operation_id.as_uuid().as_bytes());
            bytes.extend_from_slice(measure_id.as_uuid().as_bytes());
        }
    }
    bytes.extend_from_slice(value.judicial_origin.decision_id.as_uuid().as_bytes());
    bytes.extend_from_slice(value.judicial_origin.operation_id.as_uuid().as_bytes());
    let judicial = &value.last_judicial;
    bytes.extend_from_slice(judicial.owner.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(judicial.owner.decision_id.as_uuid().as_bytes());
    bytes.extend_from_slice(judicial.owner.group_digest.as_bytes());
    reference(bytes, judicial.reference);
    bytes.push(match value.last_action {
        MeasureCaptureAction::Impose => 0,
        MeasureCaptureAction::Confirm => 1,
        MeasureCaptureAction::Modify => 2,
        MeasureCaptureAction::Revoke => 3,
        MeasureCaptureAction::Cease => 4,
        MeasureCaptureAction::SubstituteOut => 5,
        MeasureCaptureAction::SubstituteIn => 6,
    });
    bytes.push(match value.validity {
        MeasureCaptureValidity::Valid => 0,
        MeasureCaptureValidity::EnteredInError => 1,
    });
    blob(bytes, &value.values.canonical_bytes())?;
    decision_wire::sources_and_projection(bytes, &value.sources, &value.projection)
}
