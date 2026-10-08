use super::{history, inconsistent, sources};
use application::{precautionary_measures::*, ApplicationError};
use domain::crypto::{DocumentHasher, Sha256Digest};
use postgres::{Row, Transaction};

pub(super) fn digest(bytes: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(&bytes).map_err(inconsistent)
}
pub(super) fn group(
    tx: &mut Transaction<'_>,
    row: &Row,
    measure_history: MeasureHistoryEvidence,
    resolved_anchor: Option<MeasureDecisionAnchorMaterial>,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureDecisionStoredOperation, ApplicationError> {
    if row.get::<_, String>("family") != "g1" {
        return Err(inconsistent("unsupported owner family"));
    }
    let super::decision_input::DecisionInput {
        case,
        context,
        command,
        format,
        actor,
        at,
    } = super::decision_input::load(tx, row, hasher)?;
    let predecessors =
        history::predecessors(&history::selections(&command.outcome), &measure_history)?;
    let result_sources = sources::result_sources(tx, case, &command, &predecessors, hasher)?;
    let material = MeasureDecisionMaterial {
        context,
        support: sources::support(tx, case, command.values.support(), format)?,
        anchor: resolved_anchor,
        predecessors,
        result_sources,
    };
    let group = prepare_measure_decision_with_history(
        hasher,
        &actor,
        case,
        command,
        material,
        &measure_history,
    )
    .map_err(inconsistent)?
    .into_group_capture(hasher, at)
    .map_err(inconsistent)?;
    if group.review.submission_digest != digest(row.get("submission_digest"))?
        || group.review.review_digest != digest(row.get("review_digest"))?
        || group.decision.capture_digest != digest(row.get("decision_digest"))?
        || group.capture_digest != digest(row.get("group_digest"))?
        || group.capture_digest != digest(row.get("owner_digest"))?
    {
        return Err(inconsistent("original group commitments differ"));
    }
    let origin = measure_group_origin(hasher, &group, &measure_history).map_err(inconsistent)?;
    Ok(MeasureDecisionStoredOperation {
        group,
        origin,
        measure_history,
    })
}

pub(super) fn member(
    row: &Row,
    capture: &MeasureCapture,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    super::member_fields::validate(
        row,
        super::member_fields::MemberFields {
            id: capture.result.id,
            revision: capture.result.revision,
            case: capture.case_id,
            operation: capture.operation_id,
            family: "m1",
            action: capture.result.action,
            values: &capture.result.values,
            digest: capture.capture_digest,
            root_operation: capture.result.origin.operation_id.as_uuid(),
        },
        hasher,
    )
}

pub(super) fn action_name(action: MeasureCaptureAction) -> &'static str {
    match action {
        MeasureCaptureAction::Impose => "impose",
        MeasureCaptureAction::Confirm => "confirm",
        MeasureCaptureAction::Modify => "modify",
        MeasureCaptureAction::Revoke => "revoke",
        MeasureCaptureAction::Cease => "cease",
        MeasureCaptureAction::SubstituteOut => "substitute_out",
        MeasureCaptureAction::SubstituteIn => "substitute_in",
    }
}
