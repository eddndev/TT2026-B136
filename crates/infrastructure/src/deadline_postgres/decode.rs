use super::{administration, dependencies, header, inconsistent, tracking};
use application::{
    deadline_evaluations::{decode_deadline_evaluation_input, decode_deadline_evaluation_record},
    deadline_profiles::{DeadlineProfileCollection, DeadlineProfileStatus},
    deadlines::*,
    ApplicationError,
};
use domain::crypto::DocumentHasher;
use postgres::{Row, Transaction};

pub(super) fn row(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineDetail, ApplicationError> {
    let head = header::row(row)?;
    let input = decode_deadline_evaluation_input(
        &row.try_get::<_, Vec<u8>>("input_canonical")
            .map_err(inconsistent)?,
    )
    .map_err(inconsistent)?;
    if input.selection.case_id != head.case_id {
        return Err(inconsistent("deadline input belongs to another case"));
    }
    let result = decode_deadline_evaluation_record(
        &row.try_get::<_, Vec<u8>>("result_canonical")
            .map_err(inconsistent)?,
    )
    .map_err(inconsistent)?;
    let profile = crate::deadline_profile_postgres::storage::detail(
        tx,
        head.profile.id,
        Some(head.profile.revision),
        hasher,
    )
    .map_err(inconsistent)?;
    if !DeadlineProfileCollection::ForCase(head.case_id).includes(profile.definition.scope())
        || profile.status != DeadlineProfileStatus::Published
    {
        return Err(inconsistent(
            "captured deadline profile has invalid scope or state",
        ));
    }
    let administration = administration::captured(tx, row, head.case_id, hasher)?;
    let heads = dependencies::read(row, &input)?;
    let material = crate::deadline_input_history::load_captured_material(
        tx,
        profile.definition.trigger(),
        &input.selection,
        input.calendar,
        &administration,
        &heads,
        hasher,
    )
    .map_err(inconsistent)?;
    let mut detail = DeadlineDetail {
        id: head.id,
        case_id: head.case_id,
        revision: head.revision,
        definition: DeadlineDefinition {
            title: head.title,
            profile: head.profile,
            input,
            responsible: head.responsible.id,
        },
        calculation: DeadlineCalculation {
            profile,
            material,
            result,
        },
        tracking: None,
        responsible: head.responsible,
        attention: head.attention,
        status: head.status,
        reason: head.reason,
        receipt: head.receipt,
        recorded_at: head.recorded_at,
        recorded_by: head.recorded_by,
    };
    detail.tracking = tracking::capture(tx, row, &detail, hasher)?;
    deadline_receipt_matches(hasher, &detail).map_err(inconsistent)?;
    if row
        .try_get::<_, serde_json::Value>("input_view")
        .map_err(inconsistent)?
        != dependencies::projection(&detail.definition.input)
        || row
            .try_get::<_, serde_json::Value>("submission_view")
            .map_err(inconsistent)?
            != header::projection(&detail)?
    {
        return Err(inconsistent(
            "deadline generated projection differs from decoded state",
        ));
    }
    let review: Vec<u8> = row.try_get("review_canonical").map_err(inconsistent)?;
    let capture: Vec<u8> = row.try_get("capture_canonical").map_err(inconsistent)?;
    let submission: Vec<u8> = row.try_get("submission_canonical").map_err(inconsistent)?;
    if deadline_review_bytes(hasher, &detail)? != review
        || deadline_capture_bytes(hasher, &detail)? != capture
        || deadline_record_submission_bytes(&detail)? != submission
        || hasher.hash_bytes(&review) != detail.receipt.review_digest
        || hasher.hash_bytes(&capture) != detail.receipt.capture_digest
        || hasher.hash_bytes(&submission) != detail.receipt.submission_digest
    {
        return Err(inconsistent("stored deadline canonical receipt differs"));
    }
    let due = detail.calculation.result.due_at();
    if row
        .try_get::<_, Option<i64>>("due_at_seconds")
        .map_err(inconsistent)?
        != due.map(|at| at.unix_timestamp())
        || row
            .try_get::<_, Option<i32>>("due_at_nanoseconds")
            .map_err(inconsistent)?
            != due.map(|at| at.nanosecond() as i32)
    {
        return Err(inconsistent(
            "deadline due projection differs from historical result",
        ));
    }
    Ok(detail)
}
