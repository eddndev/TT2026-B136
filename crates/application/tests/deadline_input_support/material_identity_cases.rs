use super::*;

#[test]
fn selected_case_family_id_revision_parent_and_agreement_must_match_exact_material() {
    let sources = [
        DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2026-01-06"))),
        DeadlineSourceDetail::Fact(Box::new(notification(1, false, 1, "2026-01-06"))),
        DeadlineSourceDetail::HearingResult(Box::new(hearing(1, false, "2026-01-06", &[0]))),
    ];
    for source in sources {
        let base = request(&source);
        let material = material(source);
        for field in 0..4 {
            let mut request = base.clone();
            let FactDeclaration::Known(reference) = &mut request.trigger.source else {
                unreachable!()
            };
            match reference {
                TriggerSourceRef::Resolution(r) => match field {
                    0 => r.id = ResolutionId::new(),
                    1 => r.revision = FactRevision::new(2).unwrap(),
                    2 => request.trigger.case_id = CaseId::new(),
                    _ => {
                        *reference = TriggerSourceRef::Notification {
                            id: NotificationId::new(),
                            revision: FactRevision::initial(),
                            resolution: *r,
                        }
                    }
                },
                TriggerSourceRef::Notification {
                    id,
                    revision,
                    resolution,
                } => match field {
                    0 => *id = NotificationId::new(),
                    1 => *revision = FactRevision::new(2).unwrap(),
                    2 => resolution.id = ResolutionId::new(),
                    _ => resolution.revision = FactRevision::new(2).unwrap(),
                },
                TriggerSourceRef::HearingResult(r) => match field {
                    0 => r.hearing_id = domain::hearings::HearingId::new(),
                    1 => r.result_id = HearingResultId::new(),
                    2 => r.revision = HearingResultRevision::new(2).unwrap(),
                    _ => r.agreement_id = Some(HearingResultAgreementId::new()),
                },
            }
            inconsistent(check_deadline_inputs(
                hasher().as_ref(),
                &request,
                &material,
            ));
        }
    }
}
