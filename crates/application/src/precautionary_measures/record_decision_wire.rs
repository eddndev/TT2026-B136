use super::{decision_wire::*, *};
use crate::{
    measure_corrections::{
        measure_administrative_record_bytes, MeasureRecordRoot, OwnedJudicialMeasure,
        OwnedMeasureRecord,
    },
    ApplicationError,
};
use domain::precautionary_hearings::PrecautionaryMeasureRef;

pub(crate) fn reference_of_record(record: &OwnedMeasureRecord) -> PrecautionaryMeasureRef {
    match record {
        OwnedMeasureRecord::Judicial(judicial) => judicial.reference(),
        OwnedMeasureRecord::Administrative { capture, .. } => PrecautionaryMeasureRef::new(
            capture.result.id,
            capture.result.revision,
            capture.capture_digest,
        ),
    }
}

pub(super) fn capture_reference(capture: &MeasureCaptureV2) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        capture.result.id,
        capture.result.revision,
        capture.capture_digest,
    )
}

fn group_reference(bytes: &mut Vec<u8>, owner: &MeasureGroupRef) {
    bytes.extend_from_slice(owner.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(owner.decision_id.as_uuid().as_bytes());
    bytes.extend_from_slice(owner.group_digest.as_bytes());
}

pub(super) fn predecessor(
    bytes: &mut Vec<u8>,
    record: &OwnedMeasureRecord,
) -> Result<(), ApplicationError> {
    match record {
        OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V1(prior)) => {
            bytes.push(0);
            group_reference(bytes, &prior.owner);
            blob(bytes, &measure_capture_bytes(&prior.capture)?)?;
            bytes.extend_from_slice(prior.capture.capture_digest.as_bytes());
        }
        OwnedMeasureRecord::Administrative { owner, capture } => {
            bytes.push(1);
            bytes.extend_from_slice(owner.operation_id.as_uuid().as_bytes());
            bytes.extend_from_slice(owner.capture_digest.as_bytes());
            blob(bytes, &measure_administrative_record_bytes(capture)?)?;
            bytes.extend_from_slice(capture.capture_digest.as_bytes());
        }
        OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V2(prior)) => {
            bytes.push(2);
            group_reference(bytes, &prior.owner);
            blob(bytes, &measure_capture_v2_bytes(&prior.capture)?)?;
            bytes.extend_from_slice(prior.capture.capture_digest.as_bytes());
        }
    }
    Ok(())
}

pub(super) fn result(
    bytes: &mut Vec<u8>,
    value: &ReviewedMeasureResultV2,
) -> Result<(), ApplicationError> {
    bytes.extend_from_slice(value.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&value.revision.get().to_be_bytes());
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
    bytes.extend_from_slice(value.effect_key.as_uuid().as_bytes());
    bytes.push(match value.action {
        MeasureCaptureAction::Impose => 0,
        MeasureCaptureAction::Confirm => 1,
        MeasureCaptureAction::Modify => 2,
        MeasureCaptureAction::Revoke => 3,
        MeasureCaptureAction::Cease => 4,
        MeasureCaptureAction::SubstituteOut => 5,
        MeasureCaptureAction::SubstituteIn => 6,
    });
    bytes.push(u8::from(value.previous.is_some()));
    if let Some(previous) = value.previous {
        reference(bytes, previous);
    }
    blob(bytes, &value.values.canonical_bytes())?;
    sources_and_projection(bytes, &value.sources, &value.projection)
}
