use application::{
    deadline_evaluations::{deadline_evaluation_input_bytes, deadline_evaluation_record_bytes},
    deadline_inputs::DeadlineInputHeads,
    deadline_reevaluation::encode_observations,
    deadlines::*,
};
use domain::{
    crypto::DocumentHasher, deadline_triggers::TriggerSourceRef, procedural_facts::FactDeclaration,
};
use infrastructure::RingSha256Hasher;
use postgres::{Error, Transaction};
use serde_json::json;
use uuid::Uuid;

/// Insert a resolution-only capture into the caller's existing transaction.
pub fn insert_revision(tx: &mut Transaction<'_>, value: &DeadlineDetail) -> Result<u64, Error> {
    deadline_receipt_matches(&RingSha256Hasher, value).unwrap();
    let heads = DeadlineInputHeads::capture(&value.calculation.material);
    let (selected, head) = match (&value.definition.input.selection.source, heads.source) {
        (
            FactDeclaration::Known(TriggerSourceRef::Resolution(selected)),
            Some(TriggerSourceRef::Resolution(head)),
        ) => (*selected, head),
        _ => panic!("worker guard fixture requires a resolution source"),
    };
    assert!(value.definition.input.calendar.is_none());
    assert!(heads.calendar.is_none());
    assert_eq!(value.attention, DeadlineAttention::Pending);
    let actor = value.recorded_by.user_id().map(|id| id.as_uuid());
    let actor_email = value.recorded_by.email();
    let input = deadline_evaluation_input_bytes(&value.definition.input).unwrap();
    let result = deadline_evaluation_record_bytes(&value.calculation.result);
    let administration = &value.calculation.material.administration;
    let admin_revision = administration
        .revision()
        .map(|revision| i64::from(revision.get()));
    let admin_bytes = administration.values().canonical_bytes();
    let admin_digest = RingSha256Hasher.hash_bytes(&admin_bytes);
    let review = deadline_review_bytes(&RingSha256Hasher, value).unwrap();
    let capture = deadline_capture_bytes(&RingSha256Hasher, value).unwrap();
    let submission = deadline_record_submission_bytes(value).unwrap();
    let tracking = value
        .tracking
        .as_ref()
        .map(|capture| deadline_tracking_capture_bytes(&RingSha256Hasher, capture))
        .transpose()
        .unwrap();
    let observations = value
        .tracking
        .as_ref()
        .map(|capture| encode_observations(&capture.observations))
        .transpose()
        .unwrap();
    let due = value.calculation.result.due_at();
    if value.receipt.action == DeadlineAction::Register {
        tx.execute(
            "INSERT INTO case_deadlines(id,case_id) VALUES($1,$2)",
            &[&value.id.as_uuid(), &value.case_id.as_uuid()],
        )?;
    }
    tx.execute(
        "INSERT INTO case_deadline_revisions(
            deadline_id,case_id,revision,title,profile_id,profile_revision,input_canonical,result_canonical,
            observed_administration_revision,observed_administration_canonical,observed_administration_digest,
            responsible_id,responsible_email,responsible_role,attention,operation_id,action,reason,
            review_canonical,review_digest,capture_canonical,capture_digest,submission_canonical,submission_digest,
            recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email,
            source_kind,source_id,source_revision,source_head_revision,due_at_seconds,due_at_nanoseconds,
            tracking_canonical,observations_canonical)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
            $21,$22,$23,$24,$25,$26,$27,$28,'resolution',$29,$30,$31,$32,$33,$34,$35)",
        &[
            &value.id.as_uuid(),
            &value.case_id.as_uuid(),
            &i64::from(value.revision.get()),
            &value.definition.title.as_str(),
            &value.definition.profile.id.as_uuid(),
            &i64::from(value.definition.profile.revision.get()),
            &input,
            &result,
            &admin_revision,
            &admin_bytes,
            &admin_digest.as_bytes().as_slice(),
            &value.responsible.id.as_uuid(),
            &value.responsible.email,
            &value.responsible.role.as_str(),
            &json!({"status":"pending"}),
            &value.receipt.operation_id.as_uuid(),
            &value.receipt.action.as_str(),
            &value.reason.as_ref().map(|reason| reason.as_str()),
            &review,
            &value.receipt.review_digest.as_bytes().as_slice(),
            &capture,
            &value.receipt.capture_digest.as_bytes().as_slice(),
            &submission,
            &value.receipt.submission_digest.as_bytes().as_slice(),
            &value.recorded_at.unix_timestamp(),
            &i32::try_from(value.recorded_at.nanosecond()).unwrap(),
            &actor,
            &actor_email,
            &selected.id.as_uuid(),
            &i64::from(selected.revision.get()),
            &i64::from(head.revision.get()),
            &due.map(|at| at.unix_timestamp()),
            &due.map(|at| i32::try_from(at.nanosecond()).unwrap()),
            &tracking,
            &observations,
        ],
    )
}

/// Bind a produced revision to its durable job without committing either write.
pub fn insert_result(
    tx: &mut Transaction<'_>,
    job_id: Uuid,
    base: &DeadlineDetail,
    next: &DeadlineDetail,
) -> Result<u64, Error> {
    tx.execute(
        "INSERT INTO deadline_reevaluation_results(
            job_id,base_revision,base_submission_digest,base_capture_digest,outcome,
            result_revision,result_submission_digest,result_capture_digest,
            checked_observations_canonical,checked_administration_revision,
            checked_administration_evidence_digest,completed_at_seconds,completed_at_nanoseconds)
        VALUES($1,$2,$3,$4,'revision',$5,$6,$7,NULL,NULL,NULL,$8,$9)",
        &[
            &job_id,
            &i64::from(base.revision.get()),
            &base.receipt.submission_digest.as_bytes().as_slice(),
            &base.receipt.capture_digest.as_bytes().as_slice(),
            &i64::from(next.revision.get()),
            &next.receipt.submission_digest.as_bytes().as_slice(),
            &next.receipt.capture_digest.as_bytes().as_slice(),
            &next.recorded_at.unix_timestamp(),
            &i32::try_from(next.recorded_at.nanosecond()).unwrap(),
        ],
    )
}
