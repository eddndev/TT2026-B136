use super::*;

#[test]
fn current_cancelled_and_exact_earlier_operations_preserve_the_complete_prefix() {
    let first = operation(40);
    let last = cancelled(&first);
    assert_eq!(
        last.capture.review.status,
        domain::hearings::HearingStatus::Cancelled
    );
    assert_eq!(last.history.captures.len(), 2);
    let actor = reader(Role::Owner);
    for saved in [&first, &last] {
        for kind in READS {
            let store = successful_store(&actor, saved, saved.clone(), kind);
            assert_eq!(
                read(&service(store, identity(&actor), clock()), saved, kind).unwrap(),
                vec![saved.clone()]
            );
        }
    }
}

#[test]
fn legacy_imposition_prefix_lifts_without_changing_original_bytes_or_origin() {
    let first = crate::precautionary_receipt_support::Fixture::schedule().capture(None, at());
    let last = crate::precautionary_receipt_support::Fixture::replace(&first)
        .capture(Some(&first), at() + time::Duration::seconds(2));
    let origin = precautionary_hearing_origin(&Hasher, &first).unwrap();
    let saved = PrecautionaryHearingRecordStoredOperation {
        capture: last.clone(),
        history: PrecautionaryHearingRecordHistoryEvidence {
            origin: origin.clone(),
            captures: vec![first.clone(), last.clone()],
            record_history: crate::record_decision_support::empty_decision_history(),
        },
    };
    let bytes = precautionary_hearing_capture_bytes(&last).unwrap();
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let store = successful_store(&actor, &saved, saved.clone(), kind);
        let loaded = read(&service(store, identity(&actor), clock()), &saved, kind)
            .unwrap()
            .remove(0);
        assert_eq!(loaded.history.origin, origin);
        assert_eq!(loaded.history.captures, vec![first.clone(), last.clone()]);
        assert_eq!(
            precautionary_hearing_capture_bytes(&loaded.capture).unwrap(),
            bytes
        );
    }
}

#[test]
fn exact_old_valid_m2_remains_readable_after_a_real_later_mark_and_terminal_is_readable() {
    use crate::decision_review_support::AdministrativeFixture;
    use application::measure_corrections::MeasureAdministrativeAction;
    let fixture = judicial_fixture();
    let group = fixture.capture();
    let history = append_v2(&fixture.history, &group);
    let saved = scheduled(
        40,
        vec![reference_v2(&group.measures[0])],
        history,
        group.recorded_at,
    );
    let mut marking = AdministrativeFixture::after(&group, &fixture.history, 25);
    marking.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = marking.capture();
    assert_eq!(
        marked.review.result.validity,
        application::measure_corrections::MeasureCaptureValidity::EnteredInError
    );
    assert!(saved
        .history
        .record_history
        .records
        .administrative
        .iter()
        .all(|a| a.origin.operation_id != marked.review.command.operation_id));
    let mut terminal = fixture;
    let old = terminal.command.outcome.changes().unwrap()[0].clone();
    let domain::precautionary_measures::MeasureEffect::Confirm { previous } = old else {
        unreachable!()
    };
    terminal.effects(vec![
        domain::precautionary_measures::MeasureEffect::Revoke { previous },
    ]);
    let revoked = terminal.capture();
    let terminal_saved = scheduled(
        50,
        vec![reference_v2(&revoked.measures[0])],
        append_v2(&terminal.history, &revoked),
        revoked.recorded_at,
    );
    let actor = reader(Role::Litigator);
    for record in [&saved, &cancelled(&saved), &terminal_saved] {
        for kind in READS {
            let store = successful_store(&actor, record, record.clone(), kind);
            assert_eq!(
                read(&service(store, identity(&actor), clock()), record, kind).unwrap(),
                vec![record.clone()]
            );
        }
    }
}

#[test]
fn corrected_record_and_m2_hearings_can_share_their_exact_original_ancestors() {
    let correction = crate::record_support::RecordFixture::initial();
    let capture = correction.capture();
    let history = MeasureDecisionRecordHistoryEvidence {
        records: crate::record_support::append_administrative(&correction.history, &capture),
        decisions: vec![],
    };
    let first = scheduled(
        10,
        vec![crate::record_support::record_reference(&capture.records[0])],
        history,
        capture.recorded_at,
    );
    let second = operation(20);
    let actor = reader(Role::Owner);
    let items = vec![first.clone(), second];
    let expected = items.clone();
    let case = first.capture.review.case_id;
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(page(case, items)));
    assert_eq!(
        service(store, identity(&actor), clock())
            .list("session", case, PrecautionaryHearingReadQuery::default())
            .unwrap()
            .items,
        expected
    );
}
