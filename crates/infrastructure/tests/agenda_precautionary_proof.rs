use super::*;
use application::precautionary_measures::*;
use domain::precautionary_measures::MeasureEffect;

#[test]
fn precautionary_agenda_loads_exact_mixed_history_and_rejects_a_missing_selected_measure() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = crate::record_fixture::setup(&mut db);
    let selected = crate::administrative_fixture::corrected_reference(&seed.corrected.capture);
    let command = crate::administrative_fixture::effect_command(
        &seed.judicial.group.review.command,
        vec![MeasureEffect::Confirm { previous: selected }],
    );
    let workflow = MeasureDecisionRecordService::new(
        crate::measure_fixture::store(&db),
        Arc::new(crate::measure_fixture::TestIdentity(seed.actor.clone())),
        crate::measure_fixture::processor(),
        Arc::new(crate::measure_fixture::FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        Arc::new(crate::measure_fixture::FixedClock(db.at)),
    );
    let review = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    let MeasureDecisionRecordReceipt::V2(decision) = workflow
        .submit(
            "session",
            db.case,
            command,
            MeasureDecisionConfirmation {
                submission_digest: review.submission_digest(),
                review_digest: review.review_digest(),
            },
        )
        .unwrap()
    else {
        panic!("expected a mixed decision")
    };
    let row = &decision.group.measures[0];
    let target =
        PrecautionaryMeasureRef::new(row.result.id, row.result.revision, row.capture_digest);
    let mut command = seed.command;
    let at = scheduled(&command);
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut command.change else {
        unreachable!()
    };
    let mut input = crate::hearing_fixture::input(values);
    input.review_targets = vec![target];
    *values = PrecautionaryHearingValues::new(input).unwrap();
    let q = query(
        at,
        20,
        AgendaKind::All,
        HearingStatusFilter::Scheduled,
        None,
    );
    assert!(store(&db).list(db.owner, q).unwrap().items.is_empty());
    let hearing = crate::record_fixture::persist(&db, seed.actor, command);
    assert_eq!(
        hearing.history.record_history.records.judicial.groups.len(),
        1
    );
    assert_eq!(
        hearing.history.record_history.records.administrative.len(),
        1
    );
    assert_eq!(hearing.history.record_history.decisions.len(), 1);
    let page = store(&db).list(db.owner, q).unwrap();
    assert_eq!(page.items.len(), 1);
    let shown = appointment(&page.items[0]);
    assert_eq!(shown.id, hearing.capture.review.command.hearing_id);
    assert_eq!(shown.revision, hearing.capture.review.result_revision);
    assert_eq!(shown.purpose, PrecautionaryHearingPurpose::Review);
    assert_eq!(shown.status, HearingStatus::Scheduled);
    assert_eq!(shown.capture_digest, hearing.capture.capture_digest);
    assert_eq!(shown.participant_count, 0);
    assert!(page.complete && page.next_after.is_none());
    assert_eq!(store(&db).list(db.owner, q).unwrap(), page);
    let existing_store = store(&db);
    db.admin
        .batch_execute(
            "SET session_replication_role=replica;
        DELETE FROM case_measure_revisions WHERE family='m2';
        SET session_replication_role=origin",
        )
        .unwrap();
    let before = audits(&mut db);
    assert!(existing_store.list(db.owner, q).is_err());
    assert_eq!(audits(&mut db), before);
}

#[test]
fn decision_linked_to_initial_hearing_keeps_one_ordinary_agenda_row_without_measure_rows() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = crate::measure_fixture::setup(&mut db);
    let mut command = crate::hearing_database_support::schedule();
    let at = (db.at + Duration::days(2)).replace_nanosecond(0).unwrap();
    let application::hearings::HearingChange::Schedule { values, .. } = &mut command.change else {
        unreachable!()
    };
    *values = crate::hearing_database_support::values(
        &at.format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
    );
    let hearing = crate::hearing_database_support::persist(
        &crate::hearing_database_support::service(&db, db.owner, Role::Owner),
        db.case,
        command,
    );
    let snapshot = &hearing.snapshot;
    let mut command = seed.command;
    command.anchor = Some(MeasureDecisionAnchorRef::Initial {
        hearing_id: snapshot.id,
        revision: snapshot.revision,
        values_digest: snapshot.values_digest,
        submission_digest: snapshot.receipt.submission_digest,
    });
    let decision = crate::measure_fixture::persist(&db, seed.actor, command);
    assert_eq!(decision.group.measures.len(), 2);
    let page = store(&db)
        .list(
            db.owner,
            query(at, 20, AgendaKind::All, HearingStatusFilter::All, None),
        )
        .unwrap();
    assert_eq!(page.items.len(), 1);
    let AgendaItem::Hearing(shown) = &page.items[0] else {
        panic!("expected the original initial hearing")
    };
    assert_eq!(shown.id, snapshot.id);
    assert_eq!(shown.revision, snapshot.revision);
    assert!(page.complete && page.next_after.is_none());
    assert!(store(&db)
        .list(
            db.owner,
            query(
                at,
                20,
                AgendaKind::PrecautionaryHearing,
                HearingStatusFilter::All,
                None,
            )
        )
        .unwrap()
        .items
        .is_empty());
}
