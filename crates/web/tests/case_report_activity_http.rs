mod case_report_http_support;
use application::case_reports::{CaseReportKind, CaseReportLitigatorQuery};
use axum::http::StatusCode;
use case_report_http_support::*;
use serde_json::{json, Value};

fn activity() -> Value {
    json!({"operation_id": ID, "report_type": "litigator_activity", "filters": {
        "occurred_from": "2026-09-01T00:00:00Z",
        "occurred_before": "2026-09-27T00:00:00Z",
        "status": "all", "author_litigator": null
    }})
}

#[tokio::test]
async fn activity_request_preserves_its_type_and_operation_period() {
    let (workflow, router) = setup(false, None);
    let (status, headers, bytes) = send(router, "POST", "", Some(activity()), true).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    private(&headers);
    let value = body(&bytes);
    assert_eq!(value["report_type"], "litigator_activity");
    assert_eq!(value["filters"], activity()["filters"]);
    assert_eq!(workflow.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn mixed_report_types_or_filter_families_never_reach_the_workflow() {
    for mutation in 0..7 {
        let mut value = activity();
        match mutation {
            0 => {
                value.as_object_mut().unwrap().remove("report_type");
            }
            1 => value["report_type"] = json!("unknown"),
            2 => value["report_type"] = json!(null),
            3 => value["filters"]["created_from"] = json!("2026-09-01T00:00:00Z"),
            4 => value["filters"]["assigned_litigator"] = json!(null),
            5 => value["filters"] = request_body()["filters"].clone(),
            _ => value["filters"]["occurred_before"] = json!("2028-01-01T00:00:00Z"),
        }
        let (workflow, router) = setup(false, None);
        let (status, headers, _) = send(router, "POST", "", Some(value), true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "mutation {mutation}");
        private(&headers);
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn activity_litigators_preserve_the_type_cursor_and_existing_public_page() {
    let (workflow, router) = setup(false, None);
    let after = "00000000-0000-0000-0000-000000000008";
    let path = format!("/litigators?limit=1&after_id={after}&report_type=litigator_activity");
    let (status, headers, bytes) = send(router, "GET", &path, None, true).await;
    assert_eq!(status, StatusCode::OK);
    private(&headers);
    assert_eq!(
        body(&bytes),
        json!({
            "scope":"office", "checked_at":"2026-09-27T00:00:00Z",
            "litigators":[{"user_id":ID,"email":"lawyer@example.test"}],
            "has_more":true,"next_after_id":ID
        })
    );
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::Litigators(CaseReportLitigatorQuery {
            kind: CaseReportKind::LitigatorActivity,
            limit: 1,
            after_id: Some(domain::identity::UserId::from_uuid(
                uuid::Uuid::parse_str(after).unwrap()
            )),
        })]
    );
}

#[tokio::test]
async fn author_picker_rejects_unknown_or_duplicate_types_before_the_workflow() {
    for query in [
        "report_type=case_state",
        "report_type=unknown",
        "report_type=",
        "report_type=litigator_activity&report_type=litigator_activity",
        "report_type=litigator_activity&scope=office",
    ] {
        let (workflow, router) = setup(false, None);
        let (status, headers, _) =
            send(router, "GET", &format!("/litigators?{query}"), None, true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
        private(&headers);
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
