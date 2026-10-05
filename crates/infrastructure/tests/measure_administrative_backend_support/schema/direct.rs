use super::*;
use domain::{
    audit::{chain_digest, AuditEvent},
    crypto::{DocumentHasher, Sha256Digest},
};
use postgres::Transaction;

fn bytes(value: &[u8]) -> String {
    format!(
        "\\x{}",
        value.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

pub(super) fn payload(capture: &MeasureAdministrativeCapture) -> Value {
    let review = &capture.review;
    let command = &review.command;
    let (action, canonical, projection, digest) = match &command.action {
        MeasureAdministrativeAction::Correct(values) => {
            let canonical = values.canonical_bytes();
            (
                "correct",
                Some(bytes(&canonical)),
                Some(infrastructure::measure_correction_codec::view(values)),
                Some(bytes(RingSha256Hasher.hash_bytes(&canonical).as_bytes())),
            )
        }
        MeasureAdministrativeAction::MarkEnteredInError => ("entered_in_error", None, None, None),
    };
    let context = review.context.material();
    json!({
        "operation_id":command.operation_id.as_uuid(),"case_id":review.case_id.as_uuid(),"action":action,
        "target_measure_id":command.target.id().as_uuid(),"target_revision":command.target.revision().get(),
        "target_capture_digest":bytes(command.target.digest().as_bytes()),"reason":command.reason.as_str(),
        "correction_canonical":canonical,"correction_view":projection,"correction_digest":digest,
        "observed_administration_revision":context.administration.revision.get(),
        "observed_stage_revision":context.stage.stage_revision().get(),
        "observed_context_digest":bytes(review.context.digest(&RingSha256Hasher).as_bytes()),
        "support_format":review.support.format.as_str(),"support_policy":review.support.policy.as_str(),
        "recorded_by":review.actor.id.as_uuid(),"recorded_by_email":review.actor.email,"recorded_by_role":review.actor.role.as_str(),
        "recorded_at_seconds":capture.recorded_at.unix_timestamp(),"recorded_at_nanoseconds":capture.recorded_at.nanosecond(),
        "submission_digest":bytes(review.submission_digest.as_bytes()),"review_digest":bytes(review.review_digest.as_bytes()),
        "capture_digest":bytes(capture.capture_digest.as_bytes()),
    })
}

pub(super) fn member(capture: &MeasureAdministrativeCapture) -> Value {
    let row = &capture.records[0];
    let result = &row.result;
    let values = result.values.canonical_bytes();
    let subject = result.values.subject();
    let (supervisor, revision) = match result.values.supervision() {
        MeasureSupervision::Known { participant, .. } => (
            Some(participant.id().as_uuid()),
            Some(participant.revision().get()),
        ),
        MeasureSupervision::Unknown { .. } => (None, None),
    };
    let action = match result.last_action {
        MeasureCaptureAction::Impose => "impose",
        MeasureCaptureAction::Confirm => "confirm",
        MeasureCaptureAction::Modify => "modify",
        MeasureCaptureAction::Revoke => "revoke",
        MeasureCaptureAction::Cease => "cease",
        MeasureCaptureAction::SubstituteOut => "substitute_out",
        MeasureCaptureAction::SubstituteIn => "substitute_in",
    };
    let validity = match result.validity {
        MeasureCaptureValidity::Valid => "valid",
        MeasureCaptureValidity::EnteredInError => "entered_in_error",
    };
    json!({
        "measure_id":result.id.as_uuid(),"revision":result.revision.get(),"case_id":row.case_id.as_uuid(),
        "owner_operation":row.operation_id.as_uuid(),"family":"c1","action":action,"validity":validity,
        "values_canonical":bytes(&values),"values_view":infrastructure::measure_decision_codec::measure_view(&result.values),
        "values_digest":bytes(RingSha256Hasher.hash_bytes(&values).as_bytes()),"capture_digest":bytes(row.capture_digest.as_bytes()),
        "subject_id":subject.id.as_uuid(),"subject_revision":subject.revision.get(),"subject_values_digest":bytes(subject.values_digest.as_bytes()),
        "supervisor_id":supervisor,"supervisor_revision":revision,
    })
}

pub(super) fn insert(
    db: &Fixture,
    capture: &MeasureAdministrativeCapture,
    payload: Option<&Value>,
    member: Option<&Value>,
    root: bool,
) -> Result<(), postgres::Error> {
    let mut client = db.runtime();
    let mut tx = client.transaction()?;
    tx.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])?;
    let sequence = audit(&mut tx, capture)?;
    tx.execute("INSERT INTO case_measure_operations(operation_id,case_id,family,owner_digest,audit_sequence)
        VALUES($1,$2,'a1',$3,$4)", &[
        &capture.review.command.operation_id.as_uuid(), &capture.review.case_id.as_uuid(),
        &capture.capture_digest.as_bytes().as_slice(), &sequence,
    ])?;
    if let Some(payload) = payload {
        tx.execute(
            "INSERT INTO case_measure_administrations
            SELECT * FROM jsonb_populate_record(NULL::case_measure_administrations,$1)",
            &[payload],
        )?;
    }
    if let Some(member) = member {
        tx.execute(
            "INSERT INTO case_measure_revisions
            SELECT * FROM jsonb_populate_record(NULL::case_measure_revisions,$1)",
            &[member],
        )?;
    }
    if root {
        tx.execute(
            "INSERT INTO case_measures(id,case_id,root_operation) VALUES($1,$2,$3)",
            &[
                &uuid::Uuid::new_v4(),
                &capture.review.case_id.as_uuid(),
                &capture.review.command.operation_id.as_uuid(),
            ],
        )?;
    }
    tx.batch_execute("SET CONSTRAINTS ALL IMMEDIATE")?;
    tx.rollback()
}

fn audit(
    tx: &mut Transaction<'_>,
    capture: &MeasureAdministrativeCapture,
) -> Result<i64, postgres::Error> {
    let head = tx.query_one(
        "SELECT sequence,chain FROM audit_events ORDER BY sequence DESC LIMIT 1",
        &[],
    )?;
    let sequence = head.get::<_, i64>(0) + 1;
    let previous = Sha256Digest::from_bytes(&head.get::<_, Vec<u8>>(1)).unwrap();
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
            review.command.target.id(),
            review.result.revision.get(),
            review.submission_digest.to_hex(),
            review.review_digest.to_hex(),
            capture.capture_digest.to_hex(),
        ),
    );
    let chain = chain_digest(&RingSha256Hasher, &previous, &event).unwrap();
    tx.execute("INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
        &[&sequence, &event.timestamp_rfc3339().unwrap(), &event.actor, &event.action, &event.resource,
            &chain.as_bytes().as_slice()])?;
    Ok(sequence)
}
