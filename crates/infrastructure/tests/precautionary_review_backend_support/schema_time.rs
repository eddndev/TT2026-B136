use super::*;
use application::cases::{CaseRepository, CaseRevisionExpectation};
use domain::{
    audit::{chain_digest, AuditEvent, GENESIS_PREVIOUS},
    crypto::Sha256Digest,
};
use postgres::{error::SqlState, Transaction};
use time::OffsetDateTime;

#[test]
fn direct_sql_hearing_cannot_predate_initial_stage_after_backdated_administration() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = crate::hearing_fixture::setup(&mut db);
    let review = service(&db, actor.clone())
        .prepare("session", db.case, command)
        .unwrap();
    let capture = prepare_precautionary_hearing_capture(
        &RingSha256Hasher,
        &actor,
        db.case,
        review.command.clone(),
        review.observed_context.clone(),
        review.sources.clone(),
        None,
    )
    .unwrap()
    .into_capture(&RingSha256Hasher, db.at)
    .unwrap();
    let administration = &review.observed_context.material().administration;
    insert_direct(&db, &capture, administration.revision.get(), db.at)
        .expect("the direct insertion accepts a valid source chronology");

    let earlier = db.at - Duration::seconds(10);
    let replaced = db
        .store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(administration.revision),
            crate::hearing_fixture::creation(&format!("BACKDATED-{}", db.case))
                .into_values()
                .editable()
                .clone(),
            earlier,
        )
        .unwrap();
    let newer = replaced.administration.snapshot().unwrap();
    assert_eq!(newer.changed_at, earlier);
    assert_eq!(newer.revision.get(), 2);
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
    // The SQL source-clock guard must reject before any receipt reconstruction.
    let result = insert_direct(&db, &capture, newer.revision.get(), earlier);
    assert_eq!(
        result
            .expect_err("accepted a hearing preceding its initial stage origin")
            .code(),
        Some(&SqlState::CHECK_VIOLATION)
    );
    assert_eq!(snapshot(&mut db), before);
}

fn insert_direct(
    db: &Fixture,
    capture: &PrecautionaryHearingCapture,
    administration_revision: u32,
    at: OffsetDateTime,
) -> Result<(), postgres::Error> {
    let mut client = db.runtime();
    let mut tx = client.transaction()?;
    tx.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])?;
    let review = &capture.review;
    let command = &review.command;
    let values = &review.resolved_values;
    tx.execute(
        "INSERT INTO case_precautionary_hearings(id,case_id) VALUES($1,$2)",
        &[&command.hearing_id.as_uuid(), &review.case_id.as_uuid()],
    )?;
    let sequence = audit(&mut tx, capture, at)?;
    tx.execute(
        "INSERT INTO case_precautionary_hearing_revisions(
            hearing_id,case_id,revision,operation_id,action,previous_capture_digest,reason,
            values_canonical,values_view,values_digest,observed_administration_revision,
            observed_stage_revision,observed_context_digest,support_format,support_policy,
            recorded_by,recorded_by_email,recorded_by_role,recorded_at_seconds,
            recorded_at_nanoseconds,submission_digest,review_digest,capture_digest,audit_sequence)
         VALUES($1,$2,1,$3,'schedule',NULL,NULL,$4,$5,sha256($4),$6,$7,$8,$9,$10,
            $11,$12,$13,$14,$15,$16,$17,$18,$19)",
        &[
            &command.hearing_id.as_uuid(),
            &review.case_id.as_uuid(),
            &command.operation_id.as_uuid(),
            &values.canonical_bytes(),
            &infrastructure::precautionary_hearing_codec::view(values),
            &i64::from(administration_revision),
            &i64::from(
                review
                    .observed_context
                    .material()
                    .stage
                    .stage_revision()
                    .get(),
            ),
            &review
                .observed_context
                .digest(&RingSha256Hasher)
                .as_bytes()
                .as_slice(),
            &review.sources.support.format.as_str(),
            &review.sources.support.policy.as_str(),
            &review.actor.id.as_uuid(),
            &review.actor.email,
            &review.actor.role.as_str(),
            &at.unix_timestamp(),
            &(at.nanosecond() as i32),
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
    let review = &capture.review;
    let marker = format!(
        "ph1:case:{}:hearing:{}:operation:{}:revision:1:submission:{}:review:{}:capture:{}",
        review.case_id,
        review.command.hearing_id,
        review.command.operation_id,
        review.submission_digest.to_hex(),
        review.review_digest.to_hex(),
        capture.capture_digest.to_hex(),
    );
    let event = AuditEvent::new(
        sequence as u64,
        at,
        &review.actor.email,
        "precautionary_hearing.schedule",
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
