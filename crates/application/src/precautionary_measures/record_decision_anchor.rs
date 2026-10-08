use super::{decision_wire::invalid, *};
use crate::{
    hearings::HearingKind,
    measure_corrections::{MeasureCaptureValidity, RecordView},
    precautionary_hearings::capture_validation,
    ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher};

pub(super) fn validate(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    command: &MeasureDecisionCommand,
    material: &MeasureDecisionMaterialV2,
    targets: &[RecordView<'_>],
) -> Result<Option<OffsetDateTime>, ApplicationError> {
    super::anchor_validation::shape(&material.anchor)?;
    let selections = super::anchor_validation::selections(&material.anchor);
    if selections.len() != targets.len() {
        return Err(invalid("anchor target inventory differs"));
    }
    let at = match (&command.anchor, &material.anchor) {
        (None, None) => return Ok(None),
        (
            Some(MeasureDecisionAnchorRef::Initial {
                hearing_id,
                revision,
                values_digest,
                submission_digest,
            }),
            Some(MeasureDecisionAnchorMaterial::Initial(detail)),
        ) => {
            let snapshot = &detail.snapshot;
            if snapshot.case_id != case_id
                || snapshot.id != *hearing_id
                || snapshot.revision != *revision
                || snapshot.values_digest != *values_digest
                || snapshot.receipt.submission_digest != *submission_digest
                || snapshot.values.kind() != HearingKind::Initial
            {
                return Err(invalid(
                    "initial hearing selection differs from exact capture",
                ));
            }
            super::anchor_validation::ordinary(hasher, detail, &material.context)?;
            snapshot.recorded_at
        }
        (
            Some(MeasureDecisionAnchorRef::Precautionary {
                hearing_id,
                revision,
                capture_digest,
            }),
            Some(MeasureDecisionAnchorMaterial::Precautionary(capture)),
        ) => {
            let review = &capture.review;
            if review.case_id != case_id
                || review.command.hearing_id != *hearing_id
                || review.result_revision != *revision
                || capture.capture_digest != *capture_digest
            {
                return Err(invalid(
                    "precautionary hearing selection differs from exact capture",
                ));
            }
            capture_validation::receipt_flat(hasher, capture)?;
            for reference in selections {
                let target = targets
                    .iter()
                    .find(|target| target.reference() == *reference)
                    .ok_or_else(|| invalid("missing exact anchor target"))?;
                if target.validity() != MeasureCaptureValidity::Valid {
                    return Err(invalid("anchor selects an entered-in-error record"));
                }
                capture_validation::context_advances(target.context(), &review.scheduling_context)?;
                if capture.recorded_at < target.recorded_at() {
                    return Err(invalid("anchor predates selected record evidence"));
                }
            }
            capture_validation::context_advances(&review.observed_context, &material.context)?;
            capture.recorded_at
        }
        _ => return Err(invalid("anchor reference and material families differ")),
    };
    Ok(Some(at))
}
