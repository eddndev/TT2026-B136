#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_currentness_support;
mod deadline_observation_support;
mod deadline_service_support;
mod deadline_support;
mod deadline_technical_support;

use application::{
    deadline_currentness::{evaluate_deadline_currentness, DeadlineCurrent, DeadlineFreshness},
    deadline_technical::DeadlineReevaluationInputs,
    deadline_tracking::{DeadlineReviewState, TrackingPolicy},
    deadlines::*,
};
use deadline_currentness_support::{checked_at, unknown_source};
use deadline_observation_support as observed;
use deadline_service_support::{identity, list_query, service, MockStore};
use deadline_technical_support::*;
use domain::{cases::CaseId, identity::Role, procedural_facts::FactLabel};
use time::UtcOffset;
use uuid::Uuid;

fn current(
    base: &DeadlineDetail,
    resolved: Option<&DeadlineReevaluationInputs>,
) -> DeadlineCurrent {
    evaluate_deadline_currentness(inputs::hasher().as_ref(), base, resolved, checked_at()).unwrap()
}

#[test]
fn overview_keeps_the_historical_calculation_separate_from_current_operational_use() {
    let accepted = accepted(TrackingPolicy::Follow);
    let fixed = accepted_with_source_metadata("fixed-source@example.test", TrackingPolicy::Fixed);
    let advanced = source_heads(&accepted, 2, false);
    let pending = revision(
        &accepted,
        event_command(source_event(&advanced, 2)),
        advanced.clone(),
    );
    let blocked = unknown_source();
    let legacy = legacy();
    let undeclared = attention(&legacy);
    let retired = retired(&accepted);
    let cases = [
        (
            current(&accepted, Some(&heads(&accepted))),
            DeadlineFreshness::Current,
        ),
        (
            current(&fixed, Some(&source_heads(&fixed, 2, false))),
            DeadlineFreshness::Current,
        ),
        (
            current(&accepted, Some(&advanced)),
            DeadlineFreshness::Changed,
        ),
        (
            current(&pending, Some(&advanced)),
            DeadlineFreshness::Current,
        ),
        (
            current(&blocked, Some(&heads(&blocked))),
            DeadlineFreshness::Current,
        ),
        (current(&legacy, None), DeadlineFreshness::NotChecked),
        (current(&undeclared, None), DeadlineFreshness::NotChecked),
        (current(&retired, None), DeadlineFreshness::NotChecked),
    ];
    for (view, freshness) in cases {
        let detail = view.detail();
        let row = DeadlineOverview::from(&view);
        assert_eq!(row.id, detail.id);
        assert_eq!(row.revision, detail.revision);
        assert_eq!(row.title, detail.definition.title);
        assert_eq!(row.responsible, detail.responsible);
        assert_eq!(row.review_state, detail.review_state());
        assert_eq!(row.calculation_due_at, detail.calculation.result.due_at());
        assert_eq!(
            row.calculation_blocked,
            detail.calculation.result.due_at().is_none()
        );
        assert_eq!(
            row.receipt_kind,
            match detail.receipt.version {
                DeadlineReceiptVersion::Legacy => DeadlineReceiptKind::Legacy,
                DeadlineReceiptVersion::Tracked(_) => DeadlineReceiptKind::Tracked,
            }
        );
        assert_eq!(row.operational, *view.operational());
        assert_eq!(row.operational.freshness(), freshness);
        assert!(row.operational.matches_overview(&row));
    }
}

#[test]
fn overview_binding_rejects_every_substituted_summary_field_and_exact_due_offset() {
    let base = accepted(TrackingPolicy::Follow);
    let row = DeadlineOverview::from(&current(&base, Some(&heads(&base))));
    assert!(row.operational.matches_overview(&row));
    for mutation in 0..12 {
        let mut changed = row.clone();
        match mutation {
            0 => changed.id = DeadlineId::new(),
            1 => changed.case_id = CaseId::new(),
            2 => changed.revision = changed.revision.next().unwrap(),
            3 => changed.receipt_kind = DeadlineReceiptKind::Legacy,
            4 => changed.status = DeadlineStatus::Retired,
            5 => changed.review_state = DeadlineReviewState::Pending,
            6 => changed.calculation_due_at = None,
            7 => {
                changed.calculation_due_at = changed
                    .calculation_due_at
                    .map(|value| value.to_offset(UtcOffset::from_hms(2, 0, 0).unwrap()))
            }
            8 => changed.calculation_blocked = true,
            9 => changed.title = FactLabel::new("Substituted title").unwrap(),
            10 => changed.responsible.email = "substituted@example.test".into(),
            _ => changed.attention_recorded = true,
        }
        assert!(
            !changed.operational.matches_overview(&changed),
            "{mutation}"
        );
    }
}

fn accepted_with_source_metadata(email: &str, policy: TrackingPolicy) -> DeadlineDetail {
    let (command, mut preparation) = deadline_support::fixture();
    let material = &mut preparation.resolved.as_mut().unwrap().material;
    for source in [&mut material.source, &mut material.source_head] {
        inputs::metadata_mut(observed::fact_mut(source))
            .recorded_by
            .email = email.into();
    }
    human(command, preparation, Some(policies(policy)), None)
}

#[test]
fn an_operational_view_from_the_same_identity_and_revision_cannot_replace_another_capture() {
    let first = accepted_with_source_metadata("first-source@example.test", TrackingPolicy::Follow);
    let second =
        accepted_with_source_metadata("second-source@example.test", TrackingPolicy::Follow);
    assert_eq!(first.id, second.id);
    assert_eq!(first.revision, second.revision);
    assert_ne!(first.receipt.capture_digest, second.receipt.capture_digest);
    let mut row = DeadlineOverview::from(&current(&first, Some(&heads(&first))));
    let other = DeadlineOverview::from(&current(&second, Some(&heads(&second))));
    assert_eq!(row.title, other.title);
    assert_eq!(row.responsible, other.responsible);
    assert_eq!(row.calculation_due_at, other.calculation_due_at);
    row.operational = other.operational;
    assert!(!row.operational.matches_overview(&row));
}

#[test]
fn a_verified_nil_identity_remains_valid_on_the_initial_page() {
    let (mut command, mut preparation) = deadline_support::fixture();
    command.deadline_id = DeadlineId::from_uuid(Uuid::nil());
    preparation.deadline_id = command.deadline_id;
    let detail =
        deadline_support::detail(&deadline_support::prepare(command, preparation).unwrap());
    let historical = DeadlineCurrent::historical(inputs::hasher().as_ref(), &detail).unwrap();
    let row = DeadlineOverview::from(&historical);
    assert!(row.operational.matches_overview(&row));
    read_page(row, true).unwrap();
}

fn read_page(
    row: DeadlineOverview,
    valid: bool,
) -> Result<DeadlinePage, application::ApplicationError> {
    let mut store = MockStore::new();
    store.expect_list().times(1).return_once(move |_, _, _, _| {
        Ok(DeadlinePage {
            deadlines: vec![row],
            has_more: false,
            next_after_id: None,
        })
    });
    let (workflow, _) = service(store, identity(Role::Owner, if valid { 2 } else { 1 }));
    workflow.list("session", inputs::case_id(), list_query(20))
}

#[test]
fn the_list_service_rejects_a_transplanted_operational_view_before_disclosure() {
    let base = accepted(TrackingPolicy::Follow);
    let valid = DeadlineOverview::from(&current(&base, Some(&heads(&base))));
    read_page(valid.clone(), true).unwrap();
    let other = attention(&base);
    let mut changed = valid;
    changed.operational = current(&other, Some(&heads(&other))).operational().clone();
    assert!(read_page(changed, false).is_err());
}
