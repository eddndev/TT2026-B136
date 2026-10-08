use super::*;
use domain::{
    audit::{chain_digest, AuditEvent, GENESIS_PREVIOUS},
    crypto::Sha256Digest,
};
use postgres::Transaction;

pub(super) fn insert(
    db: &Fixture,
    capture: &PrecautionaryHearingCapture,
) -> Result<(), postgres::Error> {
    let mut client = db.runtime();
    let mut tx = client.transaction()?;
    tx.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])?;
    let review = &capture.review;
    let command = &review.command;
    let (values, previous, reason) = match &command.change {
        PrecautionaryHearingChange::Replace {
            values,
            expected_capture_digest,
            reason,
            ..
        } => (Some(values), expected_capture_digest, reason),
        PrecautionaryHearingChange::Cancel {
            expected_capture_digest,
            reason,
            ..
        } => (None, expected_capture_digest, reason),
        PrecautionaryHearingChange::Schedule { .. } => unreachable!(),
    };
    let canonical = values.map(|value| value.canonical_bytes());
    let projection = values.map(infrastructure::precautionary_hearing_codec::view);
    let values_digest = canonical
        .as_ref()
        .map(|bytes| RingSha256Hasher.hash_bytes(bytes).as_bytes().to_vec());
    let observed = review.observed_context.material();
    let sequence = audit(&mut tx, capture)?;
    tx.execute(
        "INSERT INTO case_precautionary_hearing_revisions(
            hearing_id,case_id,revision,operation_id,action,previous_capture_digest,reason,
            values_canonical,values_view,values_digest,observed_administration_revision,
            observed_stage_revision,observed_context_digest,support_format,support_policy,
            recorded_by,recorded_by_email,recorded_by_role,recorded_at_seconds,
            recorded_at_nanoseconds,submission_digest,review_digest,capture_digest,audit_sequence)
         VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
            $21,$22,$23,$24)",
        &[
            &command.hearing_id.as_uuid(),
            &review.case_id.as_uuid(),
            &i64::from(review.result_revision.get()),
            &command.operation_id.as_uuid(),
            &command.action().as_str(),
            &previous.as_bytes().as_slice(),
            &reason.as_str(),
            &canonical,
            &projection,
            &values_digest,
            &i64::from(observed.administration.revision.get()),
            &i64::from(observed.stage.stage_revision().get()),
            &review
                .observed_context
                .digest(&RingSha256Hasher)
                .as_bytes()
                .as_slice(),
            &values.map(|_| review.sources.support.format.as_str()),
            &values.map(|_| review.sources.support.policy.as_str()),
            &review.actor.id.as_uuid(),
            &review.actor.email,
            &review.actor.role.as_str(),
            &capture.recorded_at.unix_timestamp(),
            &(capture.recorded_at.nanosecond() as i32),
            &review.submission_digest.as_bytes().as_slice(),
            &review.review_digest.as_bytes().as_slice(),
            &capture.capture_digest.as_bytes().as_slice(),
            &sequence,
        ],
    )?;
    tx.batch_execute("SET CONSTRAINTS ALL IMMEDIATE")?;
    tx.rollback()
}

fn audit(
    tx: &mut Transaction<'_>,
    capture: &PrecautionaryHearingCapture,
) -> Result<i64, postgres::Error> {
    let previous = tx.query_opt(
        "SELECT sequence,chain FROM audit_events ORDER BY sequence DESC LIMIT 1",
        &[],
    )?;
    let (sequence, digest) = match previous {
        Some(row) => (
            row.get::<_, i64>(0) + 1,
            Sha256Digest::from_bytes(&row.get::<_, Vec<u8>>(1)).unwrap(),
        ),
        None => (0, GENESIS_PREVIOUS),
    };
    let review = &capture.review;
    let marker = format!(
        "ph1:case:{}:hearing:{}:operation:{}:revision:{}:submission:{}:review:{}:capture:{}",
        review.case_id,
        review.command.hearing_id,
        review.command.operation_id,
        review.result_revision.get(),
        review.submission_digest.to_hex(),
        review.review_digest.to_hex(),
        capture.capture_digest.to_hex(),
    );
    let event = AuditEvent::new(
        sequence as u64,
        capture.recorded_at,
        &review.actor.email,
        format!("precautionary_hearing.{}", review.command.action().as_str()),
        marker,
    );
    let chain = chain_digest(&RingSha256Hasher, &digest, &event).unwrap();
    tx.execute(
        "INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
        &[&sequence, &event.timestamp_rfc3339().unwrap(), &event.actor, &event.action,
            &event.resource, &chain.as_bytes().as_slice()],
    )?;
    Ok(sequence)
}
