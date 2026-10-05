use super::*;
use application::{measure_corrections::*, precautionary_hearings::*};
use domain::{
    audit::{chain_digest, AuditEvent},
    crypto::{DocumentHasher, Sha256Digest},
    hearings::{HearingModality, HearingTime, HearingVenue},
    precautionary_hearings::*,
};
use postgres::{error::SqlState, Transaction};
use serde_json::{json, Value};
use time::{Duration, OffsetDateTime};

fn history(seed: &RecordSeed) -> MeasureDecisionRecordHistoryEvidence {
    MeasureDecisionRecordHistoryEvidence {
        records: MeasureRecordHistoryEvidence {
            judicial: MeasureHistoryEvidence {
                groups: vec![MeasureGroupEvidence {
                    origin: seed.judicial.origin.clone(),
                    capture: seed.judicial.group.clone(),
                }],
            },
            administrative: vec![MeasureAdministrativeEvidence {
                origin: seed.corrected.origin.clone(),
                capture: seed.corrected.capture.clone(),
            }],
        },
        decisions: vec![],
    }
}

fn candidate(db: &Fixture, seed: &RecordSeed) -> MeasureDecisionGroupCaptureV2 {
    let MeasureDecisionRecordReview::V2(review) = service(db, seed.actor.clone())
        .prepare("session", db.case, seed.command.clone())
        .unwrap()
    else {
        panic!("fresh command must have a V2 review")
    };
    prepare_measure_decision_with_record_history(
        &RingSha256Hasher,
        &seed.actor,
        db.case,
        review.command.clone(),
        review.material.clone(),
        &history(seed),
    )
    .unwrap()
    .into_group_capture(&RingSha256Hasher, db.at)
    .unwrap()
}

#[test]
fn direct_sql_g2_owner_requires_m2_member_family() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let group = candidate(&db, &seed);
    let before = crate::administrative_fixture::snapshot(&mut db);
    insert_group(&db, &group, "m2").expect("exact G2/M2 insertion must be admissible");
    let error = insert_group(&db, &group, "m1").expect_err("G2 accepted an M1 row");
    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    assert_eq!(crate::administrative_fixture::snapshot(&mut db), before);
}

#[test]
fn direct_sql_g2_cannot_use_an_older_corrected_predecessor() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let group = candidate(&db, &seed);
    insert_group(&db, &group, "m2").unwrap();
    let newer = crate::administrative_fixture::correction(
        corrected_reference(&seed.corrected.capture),
        seed.command.context,
        &seed.corrected.capture.review.result.values,
        "Later corrected declaration",
    );
    crate::administrative_fixture::persist(&db, seed.actor.clone(), newer);
    let before = crate::administrative_fixture::snapshot(&mut db);
    let error = insert_group(&db, &group, "m2").expect_err("accepted a stale C predecessor");
    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    assert_eq!(crate::administrative_fixture::snapshot(&mut db), before);
}

#[test]
fn direct_sql_mixed_review_requires_the_selected_c_clock_and_validity() {
    let Some(mut db) = Fixture::new() else { return };
    let (base, judicial, correction) = crate::administrative_fixture::setup(&mut db);
    db.at += Duration::seconds(1);
    let corrected = crate::administrative_fixture::persist(&db, base.actor.clone(), correction);
    let seed = RecordSeed {
        actor: base.actor,
        judicial,
        corrected,
        command: base.command,
    };
    let capture = hearing_candidate(&db, &seed);
    let before = snapshot(&mut db);
    insert_hearing(&db, &capture).expect("exact valid C review must be admissible");
    let mut early = capture.clone();
    early.recorded_at -= Duration::nanoseconds(1);
    rehash_hearing(&mut early);
    assert_eq!(
        insert_hearing(&db, &early)
            .expect_err("accepted a review before its C capture")
            .code(),
        Some(&SqlState::CHECK_VIOLATION)
    );
    assert_eq!(snapshot(&mut db), before);

    let marked = crate::administrative_fixture::persist(
        &db,
        seed.actor.clone(),
        crate::administrative_fixture::mark(
            corrected_reference(&seed.corrected.capture),
            seed.command.context,
        ),
    );
    let mut invalid = capture;
    let old = &invalid.review.resolved_values;
    let values = PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
        purpose: old.purpose(),
        scheduled_at: old.scheduled_at(),
        modality: old.modality(),
        venue: old.venue().clone(),
        note: old.note().cloned(),
        participants: old.participants().to_vec(),
        scheduling_basis: old.scheduling_basis().clone(),
        review_targets: vec![corrected_reference(&marked.capture)],
    })
    .unwrap();
    let PrecautionaryHearingChange::Schedule {
        values: command_values,
        ..
    } = &mut invalid.review.command.change
    else {
        unreachable!()
    };
    *command_values = values.clone();
    invalid.review.resolved_values = values;
    rehash_hearing(&mut invalid);
    let before = snapshot(&mut db);
    assert_eq!(
        insert_hearing(&db, &invalid)
            .expect_err("accepted an entered-in-error exact C target")
            .code(),
        Some(&SqlState::CHECK_VIOLATION)
    );
    assert_eq!(snapshot(&mut db), before);
}

fn snapshot(db: &mut Fixture) -> Value {
    json!({"measure": crate::administrative_fixture::snapshot(db),
        "hearing": db.admin.query_one("SELECT jsonb_build_array(
            (SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM case_precautionary_hearings r),
            (SELECT jsonb_agg(to_jsonb(r) ORDER BY hearing_id,revision) FROM case_precautionary_hearing_revisions r))",
            &[]).unwrap().get::<_, Value>(0)})
}

fn bytes(value: &[u8]) -> String {
    format!(
        "\\x{}",
        value.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

fn insert_group(
    db: &Fixture,
    group: &MeasureDecisionGroupCaptureV2,
    family: &str,
) -> Result<(), postgres::Error> {
    let mut client = db.runtime();
    let mut tx = client.transaction()?;
    tx.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])?;
    let r = &group.review;
    let c = &r.command;
    let marker = format!(
        "mg2:case:{}:operation:{}:decision:{}:submission:{}:review:{}:decision_digest:{}:group:{}",
        r.case_id,
        c.operation_id,
        c.decision_id,
        r.submission_digest.to_hex(),
        r.review_digest.to_hex(),
        group.decision.capture_digest.to_hex(),
        group.capture_digest.to_hex()
    );
    let sequence = audit(
        &mut tx,
        group.recorded_at,
        &r.actor.email,
        "measure_decision.recorded",
        marker,
    )?;
    tx.execute("INSERT INTO case_measure_operations(operation_id,case_id,family,owner_digest,audit_sequence)
        VALUES($1,$2,'g2',$3,$4)", &[&c.operation_id.as_uuid(), &r.case_id.as_uuid(),
        &group.capture_digest.as_bytes().as_slice(), &sequence])?;
    let values = c.values.canonical_bytes();
    let outcome = c.outcome.canonical_bytes();
    let row = json!({
        "decision_id":c.decision_id.as_uuid(),"operation_id":c.operation_id.as_uuid(),"case_id":r.case_id.as_uuid(),
        "values_canonical":bytes(&values),"values_view":infrastructure::measure_decision_codec::decision_view(&c.values),
        "values_digest":bytes(RingSha256Hasher.hash_bytes(&values).as_bytes()),
        "outcome_canonical":bytes(&outcome),"outcome_view":infrastructure::measure_decision_codec::outcome_view(&c.outcome),
        "outcome_digest":bytes(RingSha256Hasher.hash_bytes(&outcome).as_bytes()),
        "observed_administration_revision":c.context.administration_revision.get(),
        "observed_stage_revision":c.context.stage_revision.get(),"observed_context_digest":bytes(c.context.context_digest.as_bytes()),
        "support_format":r.material.support.format.as_str(),"support_policy":r.material.support.policy.as_str(),
        "recorded_by":r.actor.id.as_uuid(),"recorded_by_email":r.actor.email,"recorded_by_role":r.actor.role.as_str(),
        "recorded_at_seconds":group.recorded_at.unix_timestamp(),"recorded_at_nanoseconds":group.recorded_at.nanosecond(),
        "submission_digest":bytes(r.submission_digest.as_bytes()),"review_digest":bytes(r.review_digest.as_bytes()),
        "decision_digest":bytes(group.decision.capture_digest.as_bytes()),"group_digest":bytes(group.capture_digest.as_bytes()),
        "anchor_kind":"none",
    });
    tx.execute("INSERT INTO case_measure_decisions SELECT * FROM jsonb_populate_record(NULL::case_measure_decisions,$1)", &[&row])?;
    for capture in &group.measures {
        let result = &capture.result;
        assert!(result.previous.is_some());
        let canonical = result.values.canonical_bytes();
        let subject = result.values.subject();
        let (supervisor, revision) = match result.values.supervision() {
            MeasureSupervision::Unknown { .. } => (None, None),
            MeasureSupervision::Known { participant, .. } => (
                Some(participant.id().as_uuid()),
                Some(participant.revision().get()),
            ),
        };
        let row = json!({
            "measure_id":result.id.as_uuid(),"revision":result.revision.get(),"case_id":capture.case_id.as_uuid(),
            "owner_operation":capture.operation_id.as_uuid(),"family":family,"action":"confirm","validity":"valid",
            "values_canonical":bytes(&canonical),"values_view":infrastructure::measure_decision_codec::measure_view(&result.values),
            "values_digest":bytes(RingSha256Hasher.hash_bytes(&canonical).as_bytes()),"capture_digest":bytes(capture.capture_digest.as_bytes()),
            "subject_id":subject.id.as_uuid(),"subject_revision":subject.revision.get(),"subject_values_digest":bytes(subject.values_digest.as_bytes()),
            "supervisor_id":supervisor,"supervisor_revision":revision,
        });
        tx.execute("INSERT INTO case_measure_revisions SELECT * FROM jsonb_populate_record(NULL::case_measure_revisions,$1)", &[&row])?;
    }
    tx.batch_execute("SET CONSTRAINTS ALL IMMEDIATE")?;
    tx.rollback()
}

fn hearing_candidate(db: &Fixture, seed: &RecordSeed) -> PrecautionaryHearingCapture {
    let context = seed.corrected.capture.review.context.clone();
    let values = PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
        purpose: PrecautionaryHearingPurpose::Review,
        scheduled_at: HearingTime::new((db.at + Duration::days(1)).replace_nanosecond(0).unwrap())
            .unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Mixed source SQL review").unwrap(),
        note: None,
        participants: vec![],
        scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
            note("Declared review"),
            seed.command.values.support(),
            note("Page 1"),
        ),
        review_targets: vec![corrected_reference(&seed.corrected.capture)],
    })
    .unwrap();
    let command = PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: PrecautionaryHearingId::new(),
        change: PrecautionaryHearingChange::Schedule {
            context: seed.command.context,
            values,
        },
    };
    prepare_precautionary_hearing_with_decision_history(
        &RingSha256Hasher,
        &seed.actor,
        db.case,
        command,
        PrecautionaryHearingDecisionPreparationMaterial {
            observed_context: context,
            predecessor: None,
            decision_history: &history(seed),
            sources: PrecautionaryHearingSources {
                participants: vec![],
                support: seed.corrected.capture.review.support.clone(),
            },
        },
    )
    .unwrap()
    .into_capture(&RingSha256Hasher, db.at)
    .unwrap()
}

fn rehash_hearing(capture: &mut PrecautionaryHearingCapture) {
    let r = &mut capture.review;
    r.submission_digest = RingSha256Hasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &r.actor,
            r.case_id,
            &r.command,
            &r.resolved_values,
        )
        .unwrap(),
    );
    r.review_digest = RingSha256Hasher.hash_bytes(&precautionary_hearing_review_bytes(r).unwrap());
    capture.capture_digest =
        RingSha256Hasher.hash_bytes(&precautionary_hearing_capture_bytes(capture).unwrap());
}

fn insert_hearing(
    db: &Fixture,
    capture: &PrecautionaryHearingCapture,
) -> Result<(), postgres::Error> {
    let mut client = db.runtime();
    let mut tx = client.transaction()?;
    tx.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])?;
    let r = &capture.review;
    let c = &r.command;
    let marker = format!(
        "ph1:case:{}:hearing:{}:operation:{}:revision:1:submission:{}:review:{}:capture:{}",
        r.case_id,
        c.hearing_id,
        c.operation_id,
        r.submission_digest.to_hex(),
        r.review_digest.to_hex(),
        capture.capture_digest.to_hex()
    );
    let sequence = audit(
        &mut tx,
        capture.recorded_at,
        &r.actor.email,
        "precautionary_hearing.schedule",
        marker,
    )?;
    tx.execute(
        "INSERT INTO case_precautionary_hearings(id,case_id) VALUES($1,$2)",
        &[&c.hearing_id.as_uuid(), &r.case_id.as_uuid()],
    )?;
    let canonical = r.resolved_values.canonical_bytes();
    let row = json!({
        "hearing_id":c.hearing_id.as_uuid(),"case_id":r.case_id.as_uuid(),"revision":1,"operation_id":c.operation_id.as_uuid(),"action":"schedule",
        "values_canonical":bytes(&canonical),"values_view":infrastructure::precautionary_hearing_codec::view(&r.resolved_values),
        "values_digest":bytes(RingSha256Hasher.hash_bytes(&canonical).as_bytes()),
        "observed_administration_revision":r.observed_context.material().administration.revision.get(),
        "observed_stage_revision":r.observed_context.material().stage.stage_revision().get(),
        "observed_context_digest":bytes(r.observed_context.digest(&RingSha256Hasher).as_bytes()),
        "support_format":r.sources.support.format.as_str(),"support_policy":r.sources.support.policy.as_str(),
        "recorded_by":r.actor.id.as_uuid(),"recorded_by_email":r.actor.email,"recorded_by_role":r.actor.role.as_str(),
        "recorded_at_seconds":capture.recorded_at.unix_timestamp(),"recorded_at_nanoseconds":capture.recorded_at.nanosecond(),
        "submission_digest":bytes(r.submission_digest.as_bytes()),"review_digest":bytes(r.review_digest.as_bytes()),
        "capture_digest":bytes(capture.capture_digest.as_bytes()),"audit_sequence":sequence,
    });
    tx.execute("INSERT INTO case_precautionary_hearing_revisions SELECT * FROM jsonb_populate_record(NULL::case_precautionary_hearing_revisions,$1)", &[&row])?;
    tx.batch_execute("SET CONSTRAINTS ALL IMMEDIATE")?;
    tx.rollback()
}

fn audit(
    tx: &mut Transaction<'_>,
    at: OffsetDateTime,
    actor: &str,
    action: &str,
    marker: String,
) -> Result<i64, postgres::Error> {
    let head = tx.query_one(
        "SELECT sequence,chain FROM audit_events ORDER BY sequence DESC LIMIT 1",
        &[],
    )?;
    let sequence = head.get::<_, i64>(0) + 1;
    let previous = Sha256Digest::from_bytes(&head.get::<_, Vec<u8>>(1)).unwrap();
    let event = AuditEvent::new(sequence as u64, at, actor, action, marker);
    let chain = chain_digest(&RingSha256Hasher, &previous, &event).unwrap();
    tx.execute("INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
        &[&sequence,&event.timestamp_rfc3339().unwrap(),&event.actor,&event.action,&event.resource,&chain.as_bytes().as_slice()])?;
    Ok(sequence)
}
