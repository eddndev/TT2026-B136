use super::*;
use application::cases::CaseRevisionExpectation;
use domain::{
    audit::{chain_digest, AuditEvent, GENESIS_PREVIOUS},
    crypto::Sha256Digest,
};
use postgres::Transaction;
use time::{Duration, OffsetDateTime};

#[test]
fn direct_sql_decision_cannot_predate_initial_stage_after_backdated_administration() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let command = no_change(&seed.command);
    let review = service(&db, seed.actor.clone())
        .prepare("session", db.case, command)
        .unwrap();
    let original = prepare_measure_decision_capture(
        &RingSha256Hasher,
        &seed.actor,
        db.case,
        review.command.clone(),
        review.material.clone(),
    )
    .unwrap()
    .into_group_capture(&RingSha256Hasher, db.at)
    .unwrap();
    insert_direct(
        &db,
        &original,
        original
            .review
            .command
            .context
            .administration_revision
            .get(),
        db.at,
    )
    .expect("the same direct insertion must accept a valid source chronology");

    let next = prepare_measure_decision_capture(
        &RingSha256Hasher,
        &seed.actor,
        db.case,
        no_change(&seed.command),
        review.material,
    )
    .unwrap()
    .into_group_capture(&RingSha256Hasher, db.at)
    .unwrap();
    let earlier = db.at - Duration::seconds(10);
    let replaced = db
        .store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(seed.command.context.administration_revision),
            creation(&format!("BACKDATED-{}", db.case))
                .into_values()
                .editable()
                .clone(),
            earlier,
        )
        .unwrap();
    let administration = replaced.administration.snapshot().unwrap();
    assert_eq!(administration.changed_at, earlier);
    assert_eq!(administration.revision.get(), 2);
    let initial_at: String = db.admin.query_one(
        "SELECT a.changed_at FROM case_initial_stage_registrations s
         JOIN case_administration_revisions a ON a.case_id=s.case_id AND a.revision=s.administration_revision
         WHERE s.case_id=$1", &[&db.case.as_uuid()],
    ).unwrap().get(0);
    assert_eq!(
        OffsetDateTime::parse(&initial_at, &time::format_description::well_known::Rfc3339).unwrap(),
        db.at
    );

    let before = snapshot(&mut db);
    // SQL must reject the source chronology independently of later receipt decoding.
    let result = insert_direct(&db, &next, administration.revision.get(), earlier);
    assert!(
        result.is_err(),
        "accepted a decision preceding its initial stage origin"
    );
    assert_eq!(snapshot(&mut db), before);
}

fn insert_direct(
    db: &Fixture,
    group: &MeasureDecisionGroupCapture,
    administration_revision: u32,
    at: OffsetDateTime,
) -> Result<(), postgres::Error> {
    let mut client = db.runtime();
    let mut tx = client.transaction()?;
    tx.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])?;
    let sequence = audit(&mut tx, group, at)?;
    let review = &group.review;
    let command = &review.command;
    tx.execute(
        "INSERT INTO case_measure_operations(operation_id,case_id,family,owner_digest,audit_sequence)
         VALUES($1,$2,'g1',$3,$4)",
        &[&command.operation_id.as_uuid(), &review.case_id.as_uuid(),
            &group.capture_digest.as_bytes().as_slice(), &sequence],
    )?;
    tx.execute(
        "INSERT INTO case_measure_decisions(decision_id,operation_id,case_id,
            values_canonical,values_view,values_digest,outcome_canonical,outcome_view,outcome_digest,
            observed_administration_revision,observed_stage_revision,observed_context_digest,
            support_format,support_policy,recorded_by,recorded_by_email,recorded_by_role,
            recorded_at_seconds,recorded_at_nanoseconds,submission_digest,review_digest,decision_digest,group_digest)
         VALUES($1,$2,$3,$4,$5,sha256($4),$6,$7,sha256($6),$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21)",
        &[
            &command.decision_id.as_uuid(), &command.operation_id.as_uuid(), &review.case_id.as_uuid(),
            &command.values.canonical_bytes(), &infrastructure::measure_decision_codec::decision_view(&command.values),
            &command.outcome.canonical_bytes(), &infrastructure::measure_decision_codec::outcome_view(&command.outcome),
            &i64::from(administration_revision), &i64::from(command.context.stage_revision.get()),
            &command.context.context_digest.as_bytes().as_slice(), &review.material.support.format.as_str(),
            &review.material.support.policy.as_str(), &review.actor.id.as_uuid(), &review.actor.email,
            &review.actor.role.as_str(), &at.unix_timestamp(), &(at.nanosecond() as i32),
            &review.submission_digest.as_bytes().as_slice(), &review.review_digest.as_bytes().as_slice(),
            &group.decision.capture_digest.as_bytes().as_slice(), &group.capture_digest.as_bytes().as_slice(),
        ],
    )?;
    tx.batch_execute("SET CONSTRAINTS ALL IMMEDIATE")?;
    tx.rollback()
}

fn audit(
    tx: &mut Transaction<'_>,
    group: &MeasureDecisionGroupCapture,
    at: OffsetDateTime,
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
    let review = &group.review;
    let marker =
        format!(
        "mg1:case:{}:operation:{}:decision:{}:submission:{}:review:{}:decision_digest:{}:group:{}",
        review.case_id, review.command.operation_id, review.command.decision_id,
        review.submission_digest.to_hex(), review.review_digest.to_hex(),
        group.decision.capture_digest.to_hex(), group.capture_digest.to_hex(),
    );
    let event = AuditEvent::new(
        sequence as u64,
        at,
        &review.actor.email,
        "measure_decision.recorded",
        &marker,
    );
    let chain = chain_digest(&RingSha256Hasher, &digest, &event).unwrap();
    tx.execute(
        "INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
        &[&sequence, &event.timestamp_rfc3339().unwrap(), &event.actor, &event.action,
            &event.resource, &chain.as_bytes().as_slice()],
    )?;
    Ok(sequence)
}
