use super::{pages, tracking_response_test_support::*};
use application::{
    deadline_currentness::*, deadline_technical::DeadlineReevaluationInputs, deadlines::*,
};

#[test]
fn collection_separates_historical_calculation_from_checked_operational_state() {
    for (detail, kind) in [
        (records::fixture(), "v1"),
        (records::tracked_fixture(), "v2"),
    ] {
        let inputs = DeadlineReevaluationInputs {
            profile_head: detail.calculation.profile.clone(),
            material: detail.calculation.material.clone(),
            notification_parent_head: None,
        };
        let current = evaluate_deadline_currentness(
            &records::Hasher,
            &detail,
            Some(&inputs),
            records::instant(),
        )
        .unwrap();
        let row = DeadlineOverview::from(&current);
        let page = DeadlinePage {
            deadlines: vec![row],
            has_more: false,
            next_after_id: None,
        };
        let output = pages::page(
            page,
            case(),
            &DeadlineQuery::new(20, None, DeadlineStatusFilter::Active).unwrap(),
        )
        .unwrap();
        let row = &output["deadlines"][0];
        assert_eq!(row["receipt_kind"], kind);
        assert_eq!(
            row["review_state"],
            if kind == "v1" {
                "legacy_undeclared"
            } else {
                "accepted"
            }
        );
        assert_eq!(row["calculation_blocked"], true);
        assert!(row["calculation_due_at"].is_null());
        assert_eq!(
            row["operational"]["freshness"],
            if kind == "v1" {
                "not_checked"
            } else {
                "current"
            }
        );
        assert!(row.get("blocked").is_none());
        assert!(row.get("due_at").is_none());
    }
}

#[test]
fn collection_rejects_a_summary_changed_after_its_projection_was_bound() {
    let detail = records::tracked_fixture();
    let historical = DeadlineCurrent::historical(&records::Hasher, &detail).unwrap();
    for mutation in 0..5 {
        let mut row = DeadlineOverview::from(&historical);
        match mutation {
            0 => row.receipt_kind = DeadlineReceiptKind::Legacy,
            1 => row.calculation_due_at = Some(records::instant()),
            2 => row.calculation_blocked = false,
            3 => row.responsible.email = "substituted@example.test".into(),
            _ => row.attention_recorded = true,
        }
        let page = DeadlinePage {
            deadlines: vec![row],
            has_more: false,
            next_after_id: None,
        };
        assert!(pages::page(
            page,
            case(),
            &DeadlineQuery::new(20, None, DeadlineStatusFilter::Active).unwrap()
        )
        .is_err());
    }
}
