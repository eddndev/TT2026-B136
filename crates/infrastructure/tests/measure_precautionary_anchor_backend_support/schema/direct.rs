use super::*;
use domain::{
    audit::{chain_digest, AuditEvent},
    crypto::{DocumentHasher, Sha256Digest},
};
use postgres::Transaction;

fn bytes(value: &[u8]) -> String {
    format!(
        "\\x{}",
        value
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub(super) fn row(group: &MeasureDecisionGroupCapture) -> Value {
    let review = &group.review;
    let command = &review.command;
    let Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id,
        revision,
        capture_digest,
    }) = &command.anchor
    else {
        unreachable!()
    };
    let values = command.values.canonical_bytes();
    let outcome = command.outcome.canonical_bytes();
    let context = &review.material.context;
    json!({
        "decision_id": command.decision_id.as_uuid(), "operation_id": command.operation_id.as_uuid(),
        "case_id": review.case_id.as_uuid(),
        "values_canonical": bytes(&values), "values_view": infrastructure::measure_decision_codec::decision_view(&command.values),
        "values_digest": bytes(RingSha256Hasher.hash_bytes(&values).as_bytes()),
        "outcome_canonical": bytes(&outcome), "outcome_view": infrastructure::measure_decision_codec::outcome_view(&command.outcome),
        "outcome_digest": bytes(RingSha256Hasher.hash_bytes(&outcome).as_bytes()),
        "observed_administration_revision": context.material().administration.revision.get(),
        "observed_stage_revision": context.material().stage.stage_revision().get(),
        "observed_context_digest": bytes(context.digest(&RingSha256Hasher).as_bytes()),
        "support_format": review.material.support.format.as_str(), "support_policy": review.material.support.policy.as_str(),
        "recorded_by": review.actor.id.as_uuid(), "recorded_by_email": review.actor.email,
        "recorded_by_role": review.actor.role.as_str(),
        "recorded_at_seconds": group.recorded_at.unix_timestamp(), "recorded_at_nanoseconds": group.recorded_at.nanosecond(),
        "submission_digest": bytes(review.submission_digest.as_bytes()), "review_digest": bytes(review.review_digest.as_bytes()),
        "decision_digest": bytes(group.decision.capture_digest.as_bytes()), "group_digest": bytes(group.capture_digest.as_bytes()),
        "anchor_kind": "precautionary", "anchor_precautionary_hearing_id": hearing_id.as_uuid(),
        "anchor_revision": revision.get(), "anchor_capture_digest": bytes(capture_digest.as_bytes()),
    })
}

pub(super) fn insert(
    db: &Fixture,
    group: &MeasureDecisionGroupCapture,
    row: &Value,
) -> Result<(), postgres::Error> {
    assert!(group.measures.is_empty());
    let mut client = db.runtime();
    let mut tx = client.transaction()?;
    tx.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])?;
    let sequence = audit(&mut tx, group)?;
    tx.execute(
        "INSERT INTO case_measure_operations(operation_id,case_id,family,owner_digest,audit_sequence)
         VALUES($1,$2,'g1',$3,$4)",
        &[&group.review.command.operation_id.as_uuid(), &group.review.case_id.as_uuid(),
            &group.capture_digest.as_bytes().as_slice(), &sequence],
    )?;
    tx.execute(
        "INSERT INTO case_measure_decisions
        SELECT * FROM jsonb_populate_record(NULL::case_measure_decisions,$1)",
        &[row],
    )?;
    tx.batch_execute("SET CONSTRAINTS ALL IMMEDIATE")?;
    tx.rollback()
}

fn audit(
    tx: &mut Transaction<'_>,
    group: &MeasureDecisionGroupCapture,
) -> Result<i64, postgres::Error> {
    let head = tx.query_one(
        "SELECT sequence,chain FROM audit_events ORDER BY sequence DESC LIMIT 1",
        &[],
    )?;
    let sequence = head.get::<_, i64>(0) + 1;
    let previous = Sha256Digest::from_bytes(&head.get::<_, Vec<u8>>(1)).unwrap();
    let review = &group.review;
    let marker = format!(
        "mg1:case:{}:operation:{}:decision:{}:submission:{}:review:{}:decision_digest:{}:group:{}",
        review.case_id,
        review.command.operation_id,
        review.command.decision_id,
        review.submission_digest.to_hex(),
        review.review_digest.to_hex(),
        group.decision.capture_digest.to_hex(),
        group.capture_digest.to_hex()
    );
    let event = AuditEvent::new(
        sequence as u64,
        group.recorded_at,
        &review.actor.email,
        "measure_decision.recorded",
        marker,
    );
    let chain = chain_digest(&RingSha256Hasher, &previous, &event).unwrap();
    tx.execute("INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
        &[&sequence, &event.timestamp_rfc3339().unwrap(), &event.actor, &event.action,
            &event.resource, &chain.as_bytes().as_slice()])?;
    Ok(sequence)
}
