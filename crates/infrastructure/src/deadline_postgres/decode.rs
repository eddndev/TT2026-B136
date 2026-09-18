use super::{dependencies, header, inconsistent, port};
use application::{
    cases::CurrentCaseAdministration,
    deadline_evaluations::{decode_deadline_evaluation_input, decode_deadline_evaluation_record},
    deadline_profiles::{DeadlineProfileCollection, DeadlineProfileStatus},
    deadlines::*,
    ApplicationError,
};
use domain::{
    case_administration::CaseRevision,
    cases::{CaseId, CaseMetadata},
    crypto::DocumentHasher,
};
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
    let administration = administration(tx, row, head.case_id, hasher)?;
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
    let detail = DeadlineDetail {
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
    deadline_receipt_matches(hasher, &detail).map_err(inconsistent)?;
    let (actor, _) = header::legacy_actor(&detail.recorded_by)?;
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
        || deadline_submission_bytes(
            actor,
            detail.case_id,
            &header::command(&detail)?,
            detail.receipt.review_digest,
        ) != submission
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
fn administration(
    tx: &mut Transaction<'_>,
    row: &Row,
    case: CaseId,
    hasher: &dyn DocumentHasher,
) -> Result<CurrentCaseAdministration, ApplicationError> {
    let revision: Option<i64> = row
        .try_get("observed_administration_revision")
        .map_err(inconsistent)?;
    let captured = match revision {
        Some(revision) => CurrentCaseAdministration::Recorded(Box::new(
            crate::hearing_postgres::administration(
                tx,
                case,
                CaseRevision::new(header::counter(revision)?).map_err(inconsistent)?,
                hasher,
            )
            .map_err(inconsistent)?,
        )),
        None => {
            let baseline = tx
                .query_opt(
                    "SELECT title,reference,required_initial_revision FROM cases WHERE id=$1
                AND octet_length(title)<=800 AND octet_length(reference)<=400",
                    &[&case.as_uuid()],
                )
                .map_err(port)?
                .ok_or_else(|| {
                    inconsistent("deadline original administration absent or unbounded")
                })?;
            if baseline
                .try_get::<_, Option<i64>>("required_initial_revision")
                .map_err(inconsistent)?
                .is_some()
            {
                return Err(inconsistent(
                    "deadline invented an unrevised administration",
                ));
            }
            let title: String = baseline.try_get("title").map_err(inconsistent)?;
            let reference: String = baseline.try_get("reference").map_err(inconsistent)?;
            let metadata = CaseMetadata::new(&title, &reference).map_err(inconsistent)?;
            if metadata.title() != title || metadata.reference() != reference {
                return Err(inconsistent(
                    "noncanonical original deadline administration",
                ));
            }
            CurrentCaseAdministration::Unrevised(metadata)
        }
    };
    let bytes: Vec<u8> = row
        .try_get("observed_administration_canonical")
        .map_err(inconsistent)?;
    let digest = header::digest(
        row.try_get("observed_administration_digest")
            .map_err(inconsistent)?,
    )?;
    if captured.values().canonical_bytes() != bytes || hasher.hash_bytes(&bytes) != digest {
        return Err(inconsistent(
            "deadline administration canonical capture differs",
        ));
    }
    Ok(captured)
}
