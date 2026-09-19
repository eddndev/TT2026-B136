use super::{attention, dependencies::Columns, inconsistent, port};
use application::{
    deadline_evaluations::{deadline_evaluation_input_bytes, deadline_evaluation_record_bytes},
    deadline_reevaluation::encode_observations,
    deadlines::*,
    ApplicationError,
};
use domain::crypto::DocumentHasher;
use postgres::Transaction;

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    value: &DeadlineDetail,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    deadline_receipt_matches(hasher, value)?;
    let actor = value.recorded_by.user_id().map(|id| id.as_uuid());
    let actor_email = value.recorded_by.email();
    if value.receipt.action == DeadlineAction::Register {
        tx.execute(
            "INSERT INTO case_deadlines(id,case_id) VALUES($1,$2)",
            &[&value.id.as_uuid(), &value.case_id.as_uuid()],
        )
        .map_err(port)?;
    }
    let input = deadline_evaluation_input_bytes(&value.definition.input)?;
    let result = deadline_evaluation_record_bytes(&value.calculation.result);
    let admin = &value.calculation.material.administration;
    let admin_revision = admin.revision().map(|value| i64::from(value.get()));
    let admin_bytes = admin.values().canonical_bytes();
    let admin_digest = hasher.hash_bytes(&admin_bytes);
    let review = deadline_review_bytes(hasher, value)?;
    let capture = deadline_capture_bytes(hasher, value)?;
    let submission = deadline_record_submission_bytes(value)?;
    let tracking = value
        .tracking
        .as_ref()
        .map(|capture| deadline_tracking_capture_bytes(hasher, capture))
        .transpose()?;
    let observations = value
        .tracking
        .as_ref()
        .map(|capture| encode_observations(&capture.observations).map_err(inconsistent))
        .transpose()?;
    let dependencies = Columns::capture(value)?;
    let due = value.calculation.result.due_at();
    tx.execute("INSERT INTO case_deadline_revisions(
        deadline_id,case_id,revision,title,profile_id,profile_revision,input_canonical,result_canonical,
        observed_administration_revision,observed_administration_canonical,observed_administration_digest,
        responsible_id,responsible_email,responsible_role,attention,operation_id,action,reason,
        review_canonical,review_digest,capture_canonical,capture_digest,submission_canonical,submission_digest,
        recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email,
        source_kind,source_id,source_revision,source_head_revision,source_hearing_id,
        source_parent_resolution_id,source_parent_resolution_revision,source_head_parent_resolution_revision,
        calendar_id,calendar_revision,calendar_head_revision,due_at_seconds,due_at_nanoseconds,
        tracking_canonical,observations_canonical)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
        $21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31,$32,$33,$34,$35,$36,$37,$38,$39,$40,$41,$42,$43)",
        &[&value.id.as_uuid(), &value.case_id.as_uuid(), &i64::from(value.revision.get()), &value.definition.title.as_str(),
        &value.definition.profile.id.as_uuid(), &i64::from(value.definition.profile.revision.get()), &input, &result,
        &admin_revision, &admin_bytes, &admin_digest.as_bytes().as_slice(),
        &value.responsible.id.as_uuid(), &value.responsible.email, &value.responsible.role.as_str(), &attention::encode(&value.attention),
        &value.receipt.operation_id.as_uuid(), &value.receipt.action.as_str(), &value.reason.as_ref().map(|value| value.as_str()),
        &review, &value.receipt.review_digest.as_bytes().as_slice(), &capture, &value.receipt.capture_digest.as_bytes().as_slice(),
        &submission, &value.receipt.submission_digest.as_bytes().as_slice(),
        &value.recorded_at.unix_timestamp(), &(value.recorded_at.nanosecond() as i32), &actor, &actor_email,
        &dependencies.source_kind, &dependencies.source_id, &dependencies.source_revision, &dependencies.source_head_revision,
        &dependencies.source_hearing_id, &dependencies.source_parent_resolution_id, &dependencies.source_parent_resolution_revision,
        &dependencies.source_head_parent_resolution_revision, &dependencies.calendar_id, &dependencies.calendar_revision, &dependencies.calendar_head_revision,
        &due.map(|at| at.unix_timestamp()), &due.map(|at| at.nanosecond() as i32), &tracking, &observations]).map_err(port)?;
    Ok(())
}
