use super::*;

#[test]
fn selected_calendar_requires_exact_id_revision_and_both_materials_even_if_unused() {
    let (mut request, mut material) = unknown();
    let exact = calendar(1, false, JudicialCalendarClassification::Countable);
    request.calendar = Some(DeadlineCalendarRef {
        id: exact.id,
        revision: exact.revision,
    });
    material.calendar = Some(exact.clone());
    material.calendar_head = Some(exact.clone());
    let result = check_deadline_inputs(hasher().as_ref(), &request, &material).unwrap();
    assert_eq!(
        result.trigger().outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::UnknownSource)
    );
    for kind in 0..4 {
        let mut bad_request = request.clone();
        let mut bad = material.clone();
        match kind {
            0 => bad.calendar = None,
            1 => bad.calendar_head = None,
            2 => bad_request.calendar.as_mut().unwrap().id = JudicialCalendarId::new(),
            _ => {
                bad_request.calendar.as_mut().unwrap().revision =
                    JudicialCalendarRevision::new(2).unwrap()
            }
        }
        inconsistent(check_deadline_inputs(hasher().as_ref(), &bad_request, &bad));
    }
}

#[test]
fn unknown_source_never_hides_extra_calendar_or_corrupted_calendar_receipts() {
    let (request, material) = unknown();
    let exact = calendar(1, false, JudicialCalendarClassification::Countable);
    for (calendar, head) in [
        (Some(exact.clone()), None),
        (None, Some(exact.clone())),
        (Some(exact.clone()), Some(exact.clone())),
    ] {
        let mut bad = material.clone();
        bad.calendar = calendar;
        bad.calendar_head = head;
        inconsistent(check_deadline_inputs(hasher().as_ref(), &request, &bad));
    }
    let mut selected = request;
    selected.calendar = Some(DeadlineCalendarRef {
        id: exact.id,
        revision: exact.revision,
    });
    for (is_head, digest_kind) in [(false, false), (false, true), (true, false), (true, true)] {
        let mut bad = material.clone();
        bad.calendar = Some(exact.clone());
        bad.calendar_head = Some(calendar(
            2,
            false,
            JudicialCalendarClassification::Countable,
        ));
        let row = if is_head {
            bad.calendar_head.as_mut().unwrap()
        } else {
            bad.calendar.as_mut().unwrap()
        };
        if digest_kind {
            row.values_digest = Sha256Digest::from_array([255; 32]);
        } else {
            row.receipt.submission_digest = Sha256Digest::from_array([255; 32]);
        }
        inconsistent(check_deadline_inputs(hasher().as_ref(), &selected, &bad));
    }
}
