use super::{
    decision_wire::{bounded, invalid},
    *,
};
use crate::{
    case_stages::CaseStageEntry,
    hearings::{hearing_receipt_matches, HearingDetail, HearingKind},
    precautionary_hearings::{capture_validation, measure_evidence, PrecautionaryContext},
    ApplicationError,
};
use domain::{
    cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher,
    precautionary_hearings::PrecautionaryMeasureRef, typed_participants::ParticipantText,
};

pub(crate) fn selections(
    anchor: &Option<MeasureDecisionAnchorMaterial>,
) -> &[PrecautionaryMeasureRef] {
    match anchor {
        Some(MeasureDecisionAnchorMaterial::Precautionary(capture)) => {
            capture.review.resolved_values.review_targets()
        }
        _ => &[],
    }
}

pub(crate) fn shape(
    anchor: &Option<MeasureDecisionAnchorMaterial>,
) -> Result<(), ApplicationError> {
    match anchor {
        Some(MeasureDecisionAnchorMaterial::Initial(detail)) => bounded(detail.participants.len())?,
        Some(MeasureDecisionAnchorMaterial::Precautionary(capture)) => {
            bounded(capture.review.sources.participants.len())?;
            bounded(capture.review.participants.len())?;
        }
        None => {}
    }
    Ok(())
}

pub(super) fn validate(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    command: &MeasureDecisionCommand,
    material: &MeasureDecisionMaterial,
    proof: &CheckedMeasureTargets<'_>,
) -> Result<Option<OffsetDateTime>, ApplicationError> {
    shape(&material.anchor)?;
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
            let s = &detail.snapshot;
            if s.case_id != case_id
                || s.id != *hearing_id
                || s.revision != *revision
                || s.values_digest != *values_digest
                || s.receipt.submission_digest != *submission_digest
                || s.values.kind() != HearingKind::Initial
            {
                return Err(invalid(
                    "initial hearing selection differs from exact capture",
                ));
            }
            ordinary(hasher, detail, &material.context)?;
            s.recorded_at
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
            if capture.recorded_at < measure_evidence::target_clock(review, proof)? {
                return Err(invalid("anchor predates selected measure evidence"));
            }
            capture_validation::context_advances(&review.observed_context, &material.context)?;
            capture.recorded_at
        }
        _ => return Err(invalid("anchor reference and material families differ")),
    };
    Ok(Some(at))
}

pub(super) fn ordinary(
    hasher: &dyn DocumentHasher,
    detail: &HearingDetail,
    context: &PrecautionaryContext,
) -> Result<(), ApplicationError> {
    hearing_receipt_matches(hasher, detail)?;
    let s = &detail.snapshot;
    if s.recorded_at.offset() != time::UtcOffset::UTC
        || !(1..=9999).contains(&s.recorded_at.year())
        || s.recorded_by.email.is_empty()
        || s.recorded_by.email.trim() != s.recorded_by.email
        || s.recorded_by.email.chars().any(char::is_control)
    {
        return Err(invalid("invalid ordinary hearing provenance"));
    }
    for item in &detail.participants {
        let p = &item.overview;
        if ParticipantText::<200>::new(&p.display_name)?.as_str() != p.display_name
            || ParticipantText::<80>::new(&p.procedural_role)?.as_str() != p.procedural_role
        {
            return Err(invalid("ordinary participant projection is not normalized"));
        }
        if let Some(org) = &p.organization {
            if ParticipantText::<200>::new(org)?.as_str() != org {
                return Err(invalid("ordinary organization is not normalized"));
            }
        }
    }
    let m = context.material();
    let a = &m.administration;
    let h = &s.scheduling_context;
    let stage_digest = match &m.stage {
        CaseStageEntry::Initial(_) => None,
        CaseStageEntry::Changed(stage) => Some(stage.values_digest),
    };
    if s.recorded_administration_revision > a.revision
        || (s.recorded_administration_revision == a.revision
            && s.recorded_administration_digest != a.values_digest)
        || h.stage_revision > m.stage.stage_revision()
        || (h.stage_revision == m.stage.stage_revision()
            && (h.stage != m.stage.stage() || h.stage_digest != stage_digest))
    {
        return Err(invalid(
            "ordinary anchor context regressed or changed at one revision",
        ));
    }
    Ok(())
}
