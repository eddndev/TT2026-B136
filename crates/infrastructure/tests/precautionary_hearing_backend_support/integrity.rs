use super::*;
use application::{cases::CaseRevisionExpectation, participants::*};
use domain::{
    audit::{chain_digest, AuditEvent, GENESIS_PREVIOUS},
    crypto::{DocumentHasher, Sha256Digest},
};
use infrastructure::PostgresParticipantStore;
use std::sync::Mutex;

#[path = "integrity_support/archived.rs"]
mod archived;

fn rejected_reads_and_reopen(
    db: &mut Fixture,
    actor: &Principal,
    command: &PrecautionaryHearingCommand,
    storage: &PostgresPrecautionaryHearingStore,
) {
    let before = snapshot(db);
    assert!(
        PrecautionaryHearingReadStore::get(storage, actor, db.case, command.hearing_id, None)
            .is_err()
    );
    assert!(PrecautionaryHearingReadStore::get_operation(
        storage,
        actor,
        db.case,
        command.operation_id
    )
    .is_err());
    assert!(PostgresPrecautionaryHearingStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
    .is_err());
    assert_eq!(snapshot(db), before);
}

#[test]
fn altered_historical_commitment_actor_role_and_admission_format_fail_closed() {
    for change in [
        "submission_digest=decode(repeat('aa',32),'hex')",
        "recorded_by_role='litigator'",
        "support_format='docx'",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let (actor, command) = setup(&mut db);
        persist(&db, actor.clone(), command.clone());
        let storage = store(&db);
        db.admin
            .batch_execute("ALTER TABLE case_precautionary_hearing_revisions DISABLE TRIGGER USER")
            .unwrap();
        db.admin
            .execute(
                &format!(
                    "UPDATE case_precautionary_hearing_revisions SET {change} WHERE hearing_id=$1"
                ),
                &[&command.hearing_id.as_uuid()],
            )
            .unwrap();
        db.admin
            .batch_execute("ALTER TABLE case_precautionary_hearing_revisions ENABLE TRIGGER USER")
            .unwrap();
        rejected_reads_and_reopen(&mut db, &actor, &command, &storage);
    }
}

#[test]
fn missing_root_or_exact_audit_event_prevents_historical_disclosure_and_startup() {
    for remove_root in [true, false] {
        let Some(mut db) = Fixture::new() else { return };
        let (actor, command) = setup(&mut db);
        persist(&db, actor.clone(), command.clone());
        let storage = store(&db);
        if remove_root {
            db.admin
                .batch_execute("ALTER TABLE case_precautionary_hearings DISABLE TRIGGER ALL")
                .unwrap();
            db.admin
                .execute(
                    "DELETE FROM case_precautionary_hearings WHERE id=$1",
                    &[&command.hearing_id.as_uuid()],
                )
                .unwrap();
            db.admin
                .batch_execute("ALTER TABLE case_precautionary_hearings ENABLE TRIGGER ALL")
                .unwrap();
        } else {
            db.admin
                .batch_execute("ALTER TABLE audit_events DISABLE TRIGGER ALL")
                .unwrap();
            db.admin.execute("DELETE FROM audit_events WHERE sequence=(SELECT audit_sequence FROM case_precautionary_hearing_revisions WHERE hearing_id=$1)", &[&command.hearing_id.as_uuid()]).unwrap();
            db.admin
                .batch_execute("ALTER TABLE audit_events ENABLE TRIGGER ALL")
                .unwrap();
        }
        rejected_reads_and_reopen(&mut db, &actor, &command, &storage);
    }
}

fn review_values(value: &PrecautionaryHearingValues) -> PrecautionaryHearingValues {
    let mut values = input(value);
    values.purpose = PrecautionaryHearingPurpose::Review;
    values.review_targets = vec![PrecautionaryMeasureRef::new(
        MeasureId::new(),
        MeasureRevision::initial(),
        Sha256Digest::from_array([77; 32]),
    )];
    PrecautionaryHearingValues::new(values).unwrap()
}

fn replace_with_coherent_review(db: &mut Fixture, capture: &mut PrecautionaryHearingCapture) {
    let value = review_values(&capture.review.resolved_values);
    let context = expectation(&capture.review.observed_context);
    capture.review.command.change = PrecautionaryHearingChange::Schedule {
        context,
        values: value.clone(),
    };
    capture.review.resolved_values = value.clone();
    rewrite_capture(db, capture);
}

fn rewrite_capture(db: &mut Fixture, capture: &mut PrecautionaryHearingCapture) {
    let value = capture.review.resolved_values.clone();
    capture.review.submission_digest = RingSha256Hasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &capture.review.actor,
            capture.review.case_id,
            &capture.review.command,
            &value,
        )
        .unwrap(),
    );
    capture.review.review_digest =
        RingSha256Hasher.hash_bytes(&precautionary_hearing_review_bytes(&capture.review).unwrap());
    capture.capture_digest =
        RingSha256Hasher.hash_bytes(&precautionary_hearing_capture_bytes(capture).unwrap());
    let sequence: i64 = db
        .admin
        .query_one(
            "SELECT audit_sequence FROM case_precautionary_hearing_revisions WHERE hearing_id=$1",
            &[&capture.review.command.hearing_id.as_uuid()],
        )
        .unwrap()
        .get(0);
    let previous = if sequence == 0 {
        GENESIS_PREVIOUS
    } else {
        let bytes: Vec<u8> = db
            .admin
            .query_one(
                "SELECT chain FROM audit_events WHERE sequence=$1",
                &[&(sequence - 1)],
            )
            .unwrap()
            .get(0);
        Sha256Digest::from_bytes(&bytes).unwrap()
    };
    let marker = format!(
        "ph1:case:{}:hearing:{}:operation:{}:revision:{}:submission:{}:review:{}:capture:{}",
        db.case,
        capture.review.command.hearing_id,
        capture.review.command.operation_id,
        capture.review.result_revision.get(),
        capture.review.submission_digest.to_hex(),
        capture.review.review_digest.to_hex(),
        capture.capture_digest.to_hex()
    );
    let event = AuditEvent::new(
        sequence as u64,
        capture.recorded_at,
        &capture.review.actor.email,
        "precautionary_hearing.schedule",
        &marker,
    );
    let chain = chain_digest(&RingSha256Hasher, &previous, &event).unwrap();
    let bytes = value.canonical_bytes();
    let values_digest = RingSha256Hasher.hash_bytes(&bytes);
    db.admin.batch_execute("ALTER TABLE case_precautionary_hearing_revisions DISABLE TRIGGER USER; ALTER TABLE audit_events DISABLE TRIGGER USER").unwrap();
    db.admin.execute("UPDATE case_precautionary_hearing_revisions SET values_canonical=$1,values_view=$2,values_digest=$3,submission_digest=$4,review_digest=$5,capture_digest=$6 WHERE hearing_id=$7", &[
        &bytes, &infrastructure::precautionary_hearing_codec::view(&value),
        &values_digest.as_bytes().as_slice(), &capture.review.submission_digest.as_bytes().as_slice(),
        &capture.review.review_digest.as_bytes().as_slice(), &capture.capture_digest.as_bytes().as_slice(),
        &capture.review.command.hearing_id.as_uuid(),
    ]).unwrap();
    db.admin
        .execute(
            "UPDATE audit_events SET resource=$1,chain=$2 WHERE sequence=$3",
            &[&marker, &chain.as_bytes().as_slice(), &sequence],
        )
        .unwrap();
    db.admin.batch_execute("ALTER TABLE case_precautionary_hearing_revisions ENABLE TRIGGER USER; ALTER TABLE audit_events ENABLE TRIGGER USER").unwrap();
}

#[test]
fn review_requires_real_measure_history_even_when_all_receipt_and_audit_hashes_match() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let mut first = persist(&db, actor.clone(), command.clone());
    let storage = store(&db);
    let mut fresh = command.clone();
    fresh.hearing_id = PrecautionaryHearingId::new();
    fresh.operation_id = PrecautionaryHearingOperationId::new();
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut fresh.change else {
        unreachable!()
    };
    *values = review_values(values);
    let before = snapshot(&mut db);
    assert!(service(&db, actor.clone())
        .prepare("session", db.case, fresh)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    replace_with_coherent_review(&mut db, &mut first.capture);
    rejected_reads_and_reopen(&mut db, &actor, &command, &storage);
}

fn durable_rows(client: &mut postgres::Client) -> serde_json::Value {
    client.query_one("SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM case_precautionary_hearings h),'revisions',(SELECT jsonb_agg(to_jsonb(h) ORDER BY hearing_id,revision) FROM case_precautionary_hearing_revisions h),'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
}

#[test]
fn administration_or_selected_participant_change_after_admission_rejects_commit_without_writes() {
    for change_administration in [true, false] {
        let Some(mut db) = Fixture::new() else { return };
        let (actor, mut command) = setup(&mut db);
        let participants =
            PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
        let source = participants
            .create(
                db.owner,
                db.case,
                ParticipantId::new(),
                ParticipantValues::new("Witness", "Witness", None, None, DirectoryStatus::Active)
                    .unwrap(),
                db.at,
            )
            .unwrap();
        let PrecautionaryHearingChange::Schedule { values, .. } = &mut command.change else {
            unreachable!()
        };
        let mut changed = input(values);
        changed
            .participants
            .push(HearingParticipantRef::new(source.id, source.revision));
        *values = PrecautionaryHearingValues::new(changed).unwrap();
        let draft = service(&db, actor.clone())
            .prepare("session", db.case, command.clone())
            .unwrap();
        let repository = db.store();
        let (owner, case, at) = (db.owner, db.case, db.at);
        let url = db.admin_url.clone();
        let captured = Arc::new(Mutex::new(None));
        let observed = captured.clone();
        let workflow = service_with_format(
            &db,
            actor,
            FormatCheck(Some(Box::new(move || {
                if change_administration {
                    repository
                        .replace_administration(
                            owner,
                            case,
                            CaseRevisionExpectation::new(1),
                            creation("Changed during admission")
                                .into_values()
                                .editable()
                                .clone(),
                            at,
                        )
                        .unwrap();
                } else {
                    participants
                        .replace(
                            owner,
                            case,
                            source.id,
                            source.revision,
                            ParticipantValues::new(
                                "Edited during admission",
                                "Witness",
                                None,
                                None,
                                DirectoryStatus::Active,
                            )
                            .unwrap(),
                            at,
                        )
                        .unwrap();
                }
                *observed.lock().unwrap() = Some(durable_rows(
                    &mut postgres::Client::connect(&url, postgres::NoTls).unwrap(),
                ));
            }))),
        );
        assert!(workflow
            .submit("session", db.case, command, confirmation(&draft))
            .is_err());
        assert_eq!(
            snapshot(&mut db),
            captured
                .lock()
                .unwrap()
                .clone()
                .expect("admission callback must run")
        );
    }
}
