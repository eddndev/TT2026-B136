use super::{decision_input, decode::digest, history, inconsistent, record_sources, sources};
use application::{precautionary_measures::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::{Row, Transaction};

pub(super) fn group(
    tx: &mut Transaction<'_>,
    row: &Row,
    record_history: MeasureDecisionRecordHistoryEvidence,
    anchor: Option<MeasureDecisionAnchorMaterial>,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureDecisionRecordStoredOperation, ApplicationError> {
    if row.get::<_, String>("family") != "g2" {
        return Err(inconsistent("expected genuine G2 owner"));
    }
    let decision_input::DecisionInput {
        case,
        context,
        command,
        format,
        actor,
        at,
    } = decision_input::load(tx, row, hasher)?;
    let predecessors = record_sources::predecessors(
        hasher,
        case,
        &history::selections(&command.outcome),
        &record_history,
    )?;
    let result_sources = record_sources::result_sources(tx, case, &command, &predecessors, hasher)?;
    let material = MeasureDecisionMaterialV2 {
        context,
        support: sources::support(tx, case, command.values.support(), format)?,
        anchor,
        predecessors,
        result_sources,
    };
    let group = prepare_measure_decision_with_record_history(
        hasher,
        &actor,
        case,
        command,
        material,
        &record_history,
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
        return Err(inconsistent("original G2 commitments differ"));
    }
    let origin = measure_group_origin_v2(hasher, &group, &record_history).map_err(inconsistent)?;
    Ok(MeasureDecisionRecordStoredOperation {
        group,
        origin,
        record_history,
    })
}

pub(super) fn member(
    row: &Row,
    capture: &MeasureCaptureV2,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let root_operation = match &capture.result.record_root {
        application::measure_corrections::MeasureRecordRoot::Judicial(root) => {
            root.operation_id.as_uuid()
        }
        application::measure_corrections::MeasureRecordRoot::Administrative {
            operation_id,
            ..
        } => operation_id.as_uuid(),
    };
    super::member_fields::validate(
        row,
        super::member_fields::MemberFields {
            id: capture.result.id,
            revision: capture.result.revision,
            case: capture.case_id,
            operation: capture.operation_id,
            family: "m2",
            action: capture.result.action,
            values: &capture.result.values,
            digest: capture.capture_digest,
            root_operation,
        },
        hasher,
    )
}
