use super::*;
use domain::{
    audit::{chain_digest, AuditEvent},
    crypto::{DocumentHasher, Sha256Digest},
};
use postgres::Transaction;
use serde_json::{json, Value};

pub(super) fn insert(
    db: &Fixture,
    capture: &MeasureAdministrativeCapture,
    marked: bool,
    replacement: bool,
    root: bool,
) -> Result<(), postgres::Error> {
    let review = &capture.review;
    let new = review.replacement.as_ref().unwrap();
    let mut client = db.runtime();
    let mut tx = client.transaction()?;
    tx.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])?;
    let sequence = audit(&mut tx, capture)?;
    tx.execute("INSERT INTO case_measure_operations(operation_id,case_id,family,owner_digest,audit_sequence)
        VALUES($1,$2,'a1',$3,$4)", &[
        &review.command.operation_id.as_uuid(), &review.case_id.as_uuid(),
        &capture.capture_digest.as_bytes().as_slice(), &sequence,
    ])?;
    tx.execute(
        "INSERT INTO case_measure_administrations
        SELECT * FROM jsonb_populate_record(NULL::case_measure_administrations,$1)",
        &[&payload(capture)],
    )?;
    if root {
        tx.execute(
            "INSERT INTO case_measures(id,case_id,root_operation) VALUES($1,$2,$3)",
            &[
                &new.id.as_uuid(),
                &review.case_id.as_uuid(),
                &review.command.operation_id.as_uuid(),
            ],
        )?;
    }
    for row in &capture.records {
        let selected = if row.result.id == new.id {
            replacement
        } else {
            marked
        };
        if selected {
            tx.execute(
                "INSERT INTO case_measure_revisions
                SELECT * FROM jsonb_populate_record(NULL::case_measure_revisions,$1)",
                &[&member(row)],
            )?;
        }
    }
    tx.batch_execute("SET CONSTRAINTS ALL IMMEDIATE")?;
    tx.rollback()
}

fn payload(capture: &MeasureAdministrativeCapture) -> Value {
    let review = &capture.review;
    let command = &review.command;
    let MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
        replacement_id,
        subject,
    } = &command.action
    else {
        panic!("expected replacement selectors")
    };
    json!({
        "operation_id":command.operation_id.as_uuid(),"case_id":review.case_id.as_uuid(),
        "action":"replace_entered_in_error","target_measure_id":command.target.id().as_uuid(),
        "target_revision":command.target.revision().get(),"target_capture_digest":bytes(command.target.digest().as_bytes()),
        "reason":command.reason.as_str(),"replacement_measure_id":replacement_id.as_uuid(),
        "replacement_subject_id":subject.id.as_uuid(),"replacement_subject_revision":subject.revision.get(),
        "replacement_subject_values_digest":bytes(subject.values_digest.as_bytes()),
        "observed_administration_revision":command.context.administration_revision.get(),
        "observed_stage_revision":command.context.stage_revision.get(),
        "observed_context_digest":bytes(command.context.context_digest.as_bytes()),
        "support_format":review.support.format.as_str(),"support_policy":review.support.policy.as_str(),
        "recorded_by":review.actor.id.as_uuid(),"recorded_by_email":review.actor.email,"recorded_by_role":review.actor.role.as_str(),
        "recorded_at_seconds":capture.recorded_at.unix_timestamp(),"recorded_at_nanoseconds":capture.recorded_at.nanosecond(),
        "submission_digest":bytes(review.submission_digest.as_bytes()),"review_digest":bytes(review.review_digest.as_bytes()),
        "capture_digest":bytes(capture.capture_digest.as_bytes()),
    })
}

fn member(row: &MeasureAdministrativeRecordCapture) -> Value {
    let result = &row.result;
    assert_eq!(result.last_action, MeasureCaptureAction::Impose);
    let canonical = result.values.canonical_bytes();
    let subject = result.values.subject();
    let (supervisor, revision) = match result.values.supervision() {
        MeasureSupervision::Known { participant, .. } => (
            Some(participant.id().as_uuid()),
            Some(participant.revision().get()),
        ),
        MeasureSupervision::Unknown { .. } => (None, None),
    };
    let validity = match result.validity {
        MeasureCaptureValidity::Valid => "valid",
        MeasureCaptureValidity::EnteredInError => "entered_in_error",
    };
    json!({
        "measure_id":result.id.as_uuid(),"revision":result.revision.get(),"case_id":row.case_id.as_uuid(),
        "owner_operation":row.operation_id.as_uuid(),"family":"c1","action":"impose","validity":validity,
        "values_canonical":bytes(&canonical),"values_view":infrastructure::measure_decision_codec::measure_view(&result.values),
        "values_digest":bytes(RingSha256Hasher.hash_bytes(&canonical).as_bytes()),"capture_digest":bytes(row.capture_digest.as_bytes()),
        "subject_id":subject.id.as_uuid(),"subject_revision":subject.revision.get(),"subject_values_digest":bytes(subject.values_digest.as_bytes()),
        "supervisor_id":supervisor,"supervisor_revision":revision,
    })
}

fn audit(
    tx: &mut Transaction<'_>,
    capture: &MeasureAdministrativeCapture,
) -> Result<i64, postgres::Error> {
    let row = tx.query_one(
        "SELECT sequence,chain FROM audit_events ORDER BY sequence DESC LIMIT 1",
        &[],
    )?;
    let sequence = row.get::<_, i64>(0) + 1;
    let previous = Sha256Digest::from_bytes(&row.get::<_, Vec<u8>>(1)).unwrap();
    let review = &capture.review;
    let event = AuditEvent::new(
        sequence as u64,
        capture.recorded_at,
        &review.actor.email,
        "measure_administrative.recorded",
        format!(
            "ma1:case:{}:operation:{}:measure:{}:revision:{}:submission:{}:review:{}:capture:{}",
            review.case_id,
            review.command.operation_id,
            review.result.id,
            review.result.revision.get(),
            review.submission_digest.to_hex(),
            review.review_digest.to_hex(),
            capture.capture_digest.to_hex(),
        ),
    );
    let digest = chain_digest(&RingSha256Hasher, &previous, &event).unwrap();
    tx.execute("INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
        &[&sequence, &event.timestamp_rfc3339().unwrap(), &event.actor, &event.action, &event.resource,
            &digest.as_bytes().as_slice()])?;
    Ok(sequence)
}

fn bytes(value: &[u8]) -> String {
    format!(
        "\\x{}",
        value
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}
