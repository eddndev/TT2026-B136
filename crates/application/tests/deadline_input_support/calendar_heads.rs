use super::*;

fn with_calendar(
    exact: JudicialCalendarDetail,
    head: JudicialCalendarDetail,
) -> (DeadlineInputRequest, DeadlineInputMaterial) {
    let source = DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2026-01-06")));
    let mut request = request(&source);
    request.rule = ArithmeticRule::Days {
        quantity: NonZeroU32::new(1).unwrap(),
        inclusion: DayInclusion::OnAnchor,
        basis: DayBasis::CalendarCountable,
        final_day: FinalDayPolicy::Preserve,
    };
    request.calendar = Some(DeadlineCalendarRef {
        id: exact.id,
        revision: exact.revision,
    });
    let mut material = material(source);
    material.calendar = Some(exact);
    material.calendar_head = Some(head);
    (request, material)
}
#[test]
fn changed_retired_calendar_head_cannot_replace_selected_classifications() {
    let exact = calendar(1, false, JudicialCalendarClassification::Countable);
    let head = calendar(3, true, JudicialCalendarClassification::Excluded);
    let (request, material) = with_calendar(exact.clone(), head);
    let before = material.clone();
    let result = check_deadline_inputs(hasher().as_ref(), &request, &material).unwrap();
    assert_eq!(
        result.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: "2026-01-06".parse().unwrap()
        }
    );
    assert_eq!(material.calendar.as_ref().unwrap(), &exact);
    assert_eq!(material, before);
}
#[test]
fn selected_retired_calendar_is_not_implicitly_ineligible() {
    let exact = calendar(3, true, JudicialCalendarClassification::Countable);
    let (request, material) = with_calendar(exact.clone(), exact);
    let result = check_deadline_inputs(hasher().as_ref(), &request, &material).unwrap();
    assert_eq!(
        result.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: "2026-01-06".parse().unwrap()
        }
    );
}
#[test]
fn calendar_head_cannot_be_older_or_belong_to_another_root() {
    let exact = calendar(2, false, JudicialCalendarClassification::Countable);
    for mut head in [
        calendar(1, false, JudicialCalendarClassification::Countable),
        calendar(3, false, JudicialCalendarClassification::Countable),
    ] {
        if head.revision.get() == 3 {
            head.id = JudicialCalendarId::new();
            resign_calendar(&mut head);
        }
        judicial_calendar_receipt_matches(hasher().as_ref(), &head).unwrap();
        let (request, material) = with_calendar(exact.clone(), head);
        inconsistent(check_deadline_inputs(
            hasher().as_ref(),
            &request,
            &material,
        ));
    }
}
#[test]
fn calendar_head_scope_and_same_revision_projection_are_checked_beyond_receipt() {
    let exact = calendar(1, false, JudicialCalendarClassification::Countable);
    let mut same = exact.clone();
    same.recorded_by.email = "changed@example.test".into();
    let mut changed_scope = calendar(2, false, JudicialCalendarClassification::Countable);
    let old = changed_scope.values.scope();
    let codes = old
        .entity_codes()
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let scope = JudicialCalendarScope::new(JudicialCalendarScopeInput {
        title: "Different immutable scope",
        jurisdiction: old.jurisdiction(),
        entity_codes: &codes,
        authority: old.authority(),
        organ: old.organ(),
        territory: old.territory(),
        use_description: old.use_description(),
    })
    .unwrap();
    changed_scope.values = JudicialCalendarValues::new(
        scope,
        changed_scope.values.coverage(),
        changed_scope.values.sources().to_vec(),
        changed_scope.values.weekly_pattern().to_vec(),
        changed_scope.values.exceptions().to_vec(),
    )
    .unwrap();
    resign_calendar(&mut changed_scope);
    for head in [same, changed_scope] {
        judicial_calendar_receipt_matches(hasher().as_ref(), &head).unwrap();
        let (request, material) = with_calendar(exact.clone(), head);
        inconsistent(check_deadline_inputs(
            hasher().as_ref(),
            &request,
            &material,
        ));
    }
}
