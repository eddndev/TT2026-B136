use super::*;

fn rejects(
    request: &PrecautionaryHearingRecordStoredOperation,
    changed: PrecautionaryHearingRecordStoredOperation,
) {
    let actor = reader(Role::Litigator);
    for kind in READS {
        let store = successful_store(&actor, request, changed.clone(), kind);
        assert!(read(&service(store, identity(&actor), clock()), request, kind).is_err());
    }
}

#[test]
fn origin_prefix_capture_and_exact_mixed_owner_closure_are_all_required() {
    let saved = cancelled(&operation(40));
    for change in 0..9 {
        let mut changed = saved.clone();
        match change {
            0 => changed.history.origin.operation_id = PrecautionaryHearingOperationId::new(),
            1 => {
                changed.history.captures.remove(0);
            }
            2 => {
                changed.history.captures[0].capture_digest =
                    Sha256Digest::from_bytes(&[9; 32]).unwrap()
            }
            3 => changed.capture.recorded_at += time::Duration::nanoseconds(1),
            4 => changed.history.record_history.decisions.clear(),
            5 => changed
                .history
                .record_history
                .records
                .administrative
                .clear(),
            6 => changed
                .history
                .record_history
                .records
                .judicial
                .groups
                .clear(),
            7 => changed
                .history
                .record_history
                .decisions
                .push(changed.history.record_history.decisions[0].clone()),
            _ => {
                changed.history.record_history.decisions[0].capture.measures[0].recorded_at +=
                    time::Duration::nanoseconds(1)
            }
        }
        rejects(&saved, changed);
    }
}

#[test]
fn rehashed_own_receipt_cannot_rewrite_source_provenance_or_conceal_wrong_returned_capture() {
    let saved = operation(40);
    let mut changed = saved.clone();
    changed.capture.review.sources.support.name = "another-valid-appointment.pdf".into();
    refresh(&mut changed.capture);
    assert_ne!(changed.capture.capture_digest, saved.capture.capture_digest);
    rejects(&saved, changed);
    let mut changed = saved.clone();
    changed.history.record_history.records.administrative[0]
        .capture
        .review
        .support
        .name = "invented ancestor support.pdf".into();
    use domain::crypto::DocumentHasher;
    let owner = &mut changed.history.record_history.records.administrative[0];
    owner.capture.review.review_digest = Hasher.hash_bytes(
        &application::measure_corrections::measure_administrative_review_bytes(
            &owner.capture.review,
        )
        .unwrap(),
    );
    owner.capture.capture_digest = Hasher.hash_bytes(
        &application::measure_corrections::measure_administrative_capture_bytes(&owner.capture)
            .unwrap(),
    );
    owner.origin.capture_digest = owner.capture.capture_digest;
    rejects(&saved, changed);
}

#[test]
fn forged_hearing_explicitly_selecting_an_entered_in_error_c_is_rejected() {
    use crate::decision_review_support::{
        append_administrative_decision_history, AdministrativeFixture,
    };
    use application::measure_corrections::MeasureAdministrativeAction;
    let fixture = judicial_fixture();
    let group = fixture.capture();
    let original = scheduled(
        40,
        vec![reference_v2(&group.measures[0])],
        append_v2(&fixture.history, &group),
        group.recorded_at,
    );
    let mut marking = AdministrativeFixture::after(&group, &fixture.history, 77);
    marking.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = marking.capture();
    let history = append_administrative_decision_history(&marking.history, &marked);
    let mut changed = original.clone();
    let mut input =
        crate::record_review_support::values_input(&changed.capture.review.resolved_values);
    input.review_targets = vec![crate::record_support::record_reference(&marked.records[0])];
    let values = PrecautionaryHearingValues::new(input).unwrap();
    let PrecautionaryHearingChange::Schedule {
        values: command_values,
        ..
    } = &mut changed.capture.review.command.change
    else {
        unreachable!()
    };
    *command_values = values.clone();
    changed.capture.review.resolved_values = values;
    changed.capture.recorded_at = marked.recorded_at;
    refresh(&mut changed.capture);
    changed.history.captures = vec![changed.capture.clone()];
    changed.history.origin.submission_digest = changed.capture.review.submission_digest;
    changed.history.origin.review_digest = changed.capture.review.review_digest;
    changed.history.origin.capture_digest = changed.capture.capture_digest;
    changed.history.record_history = history;
    rejects(&original, changed);
}
