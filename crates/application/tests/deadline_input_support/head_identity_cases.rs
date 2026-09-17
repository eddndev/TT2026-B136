use super::*;

#[test]
fn a_self_consistent_head_with_a_foreign_case_or_identity_is_rejected() {
    for exact in sources(1, false, "2026-01-06") {
        for change_case in [false, true] {
            let request = request(&exact);
            let mut head = exact.clone();
            match &mut head {
                DeadlineSourceDetail::Fact(detail) => {
                    match &mut detail.snapshot {
                        ProceduralFactSnapshot::Resolution(s) => {
                            s.root = ResolutionRoot::new(
                                if change_case {
                                    s.root.id()
                                } else {
                                    ResolutionId::new()
                                },
                                if change_case {
                                    CaseId::new()
                                } else {
                                    s.root.case_id()
                                },
                            )
                        }
                        ProceduralFactSnapshot::Notification(s) => {
                            s.root = NotificationRoot::new(
                                if change_case {
                                    s.root.id()
                                } else {
                                    NotificationId::new()
                                },
                                if change_case {
                                    CaseId::new()
                                } else {
                                    s.root.case_id()
                                },
                                s.root.resolution_id(),
                            )
                        }
                    }
                    if change_case {
                        let case = detail.snapshot.case_id();
                        if let Some(parent) = &mut detail.sources.resolved.resolution {
                            parent.case_id = case;
                        }
                    }
                    resign_fact(detail);
                }
                DeadlineSourceDetail::HearingResult(detail) => {
                    if change_case {
                        detail.snapshot.case_id = CaseId::new();
                    } else {
                        detail.snapshot.id = HearingResultId::new();
                    }
                    resign_hearing(detail);
                }
            }
            validate_source(&head);
            let mut material = material(exact.clone());
            material.source_head = Some(head);
            inconsistent(check_deadline_inputs(
                hasher().as_ref(),
                &request,
                &material,
            ));
        }
    }
}
#[test]
fn notification_head_cannot_change_parent_identity_even_with_a_consistent_receipt() {
    let exact = DeadlineSourceDetail::Fact(Box::new(notification(1, false, 1, "2026-01-06")));
    let request = request(&exact);
    let mut head = notification(2, false, 2, "2026-01-09");
    let parent_id = ResolutionId::new();
    let ProceduralFactSnapshot::Notification(snapshot) = &mut head.snapshot else {
        unreachable!()
    };
    let mut input = notification_input(&snapshot.values);
    input.resolution.id = parent_id;
    snapshot.values = NotificationValues::new(input).unwrap();
    snapshot.root = NotificationRoot::new(snapshot.root.id(), snapshot.root.case_id(), parent_id);
    head.sources
        .resolved
        .resolution
        .as_mut()
        .unwrap()
        .reference
        .id = parent_id;
    head.sources.views.resolution.as_mut().unwrap().reference.id = parent_id;
    resign_fact(&mut head);
    fact_receipt_matches(hasher().as_ref(), &head).unwrap();
    let mut material = material(exact);
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(head)));
    inconsistent(check_deadline_inputs(
        hasher().as_ref(),
        &request,
        &material,
    ));
}
