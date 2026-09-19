mod agenda_support;
#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_currentness_support;
mod deadline_observation_support;
mod deadline_support;
mod deadline_technical_support;

use agenda_support::*;
use application::{
    agenda::*,
    deadline_currentness::{evaluate_deadline_currentness, DeadlineCurrent},
    deadline_technical::DeadlineReevaluationInputs,
    deadline_tracking::TrackingPolicy,
    deadlines::*,
};
use deadline_technical_support::*;
use domain::{case_administration::CaseAdministrativeStatus, cases::CaseId};
use time::{Duration, UtcOffset};

fn current(
    base: &DeadlineDetail,
    resolved: Option<&DeadlineReevaluationInputs>,
) -> DeadlineCurrent {
    evaluate_deadline_currentness(inputs::hasher().as_ref(), base, resolved, checked_at()).unwrap()
}

fn entry(view: &DeadlineCurrent) -> AgendaItem {
    let deadline = DeadlineOverview::from(view);
    AgendaItem::Deadline {
        case: AgendaCaseSummary {
            case_id: deadline.case_id,
            title: "Authorized case".into(),
            reference: "CASE-1".into(),
            status: CaseAdministrativeStatus::Active,
        },
        deadline: Box::new(deadline),
    }
}

#[test]
fn mixed_order_uses_operational_instants_and_keeps_attention_and_closed_cases() {
    let base = attention(&accepted(TrackingPolicy::Fixed));
    let view = current(&base, Some(&source_heads(&base, 2, false)));
    let mut due = entry(&view);
    let AgendaItem::Deadline { case, deadline } = &mut due else {
        unreachable!()
    };
    case.status = CaseAdministrativeStatus::Closed;
    assert!(deadline.attention_recorded);
    let at = deadline.operational.due_at().unwrap();
    let hearing = AgendaItem::Hearing(hearing(at, deadline.id.as_uuid().as_u128()));
    let original = deadline.calculation_due_at;
    let result = page(vec![hearing, due]);
    assert!(result.validate(&query(2)).is_ok());
    assert!(result.items[0].key().unwrap() < result.items[1].key().unwrap());
    assert_eq!(
        result.items[1].key().unwrap().at(),
        at.to_offset(UtcOffset::UTC)
    );
    let AgendaItem::Deadline { deadline, .. } = &result.items[1] else {
        unreachable!()
    };
    assert_eq!(deadline.calculation_due_at, original);
    assert_eq!(deadline.operational.checked_at(), Some(result.checked_at));
}

#[test]
fn historical_pending_retired_changed_and_blocked_calculations_cannot_supply_agenda_dates() {
    let base = accepted(TrackingPolicy::Follow);
    let advanced = source_heads(&base, 2, false);
    let pending = revision(
        &base,
        event_command(source_event(&advanced, 2)),
        advanced.clone(),
    );
    let blocked = deadline_currentness_support::unknown_source();
    let old = legacy();
    let withdrawn = retired(&base);
    let values = [
        DeadlineCurrent::historical(inputs::hasher().as_ref(), &base).unwrap(),
        current(&base, Some(&advanced)),
        current(&pending, Some(&advanced)),
        current(&blocked, Some(&heads(&blocked))),
        current(&old, None),
        current(&withdrawn, None),
    ];
    for value in values {
        assert!(value.operational().due_at().is_none());
        let row = entry(&value);
        assert!(row.key().is_err());
        assert!(page(vec![row]).validate(&query(1)).is_err());
    }
    let valid = entry(&current(&base, Some(&heads(&base))));
    assert!(page(vec![valid]).validate(&query(1)).is_ok());
}

#[test]
fn deadline_binding_case_metadata_and_shared_observation_time_are_required() {
    let base = accepted(TrackingPolicy::Follow);
    let view = current(&base, Some(&heads(&base)));
    for mutation in 0..7 {
        let mut value = entry(&view);
        let AgendaItem::Deadline { case, deadline } = &mut value else {
            unreachable!()
        };
        match mutation {
            0 => case.case_id = CaseId::new(),
            1 => case.title = " padded".into(),
            2 => deadline.receipt_kind = DeadlineReceiptKind::Legacy,
            3 => deadline.responsible.email = "other@example.test".into(),
            4 => {
                deadline.calculation_due_at = deadline
                    .calculation_due_at
                    .map(|at| at.to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()))
            }
            5 => deadline.calculation_blocked = true,
            _ => deadline.revision = deadline.revision.next().unwrap(),
        }
        assert!(
            page(vec![value]).validate(&query(1)).is_err(),
            "mutation {mutation}"
        );
    }
    let mut result = page(vec![entry(&view)]);
    result.checked_at += Duration::nanoseconds(1);
    assert!(result.validate(&query(1)).is_err());
    result.checked_at = checked_at();
    assert!(result.validate(&query(1)).is_ok());
}
