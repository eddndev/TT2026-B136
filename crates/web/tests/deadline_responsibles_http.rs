mod deadline_http_support;
use application::deadlines::*;
use deadline_http_support::*;
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

fn id(n: u128) -> UserId {
    UserId::from_uuid(Uuid::from_u128(n))
}
fn page() -> DeadlineResponsiblePage {
    DeadlineResponsiblePage {
        case_id: case(),
        responsibles: vec![DeadlineResponsibleCandidate {
            id: id(1),
            email: "staff@example.test".into(),
            role: Role::Paralegal,
        }],
        has_more: true,
        next_after_id: Some(id(1)),
    }
}

#[tokio::test]
async fn responsible_static_route_has_exact_minimal_projection_and_preserves_nil_cursor() {
    let workflow = Arc::new(Workflow::default());
    *workflow.responsible_page.lock().unwrap() = Some(page());
    let (status, body) = request(
        workflow.clone(),
        "GET",
        &format!("{BASE}/responsibles?limit=1&after_id={}", id(0)),
        Some("paralegal"),
        None,
        &[],
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({"case_id":CASE,"responsibles":[{"id":id(1),"email":"staff@example.test","role":"paralegal"}],"has_more":true,"next_after_id":id(1)})
    );
    assert_eq!(
        workflow.calls.lock().unwrap()[0],
        json!(["paralegal", CASE, ["responsibles", 1, id(0)]])
    );
    let workflow = Arc::new(Workflow::default());
    let (status, body) = request(
        workflow.clone(),
        "GET",
        &format!("{BASE}/responsibles"),
        Some("owner"),
        None,
        &[],
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({"case_id":CASE,"responsibles":[],"has_more":false,"next_after_id":null})
    );
    assert_eq!(
        workflow.calls.lock().unwrap()[0],
        json!(["owner", CASE, ["responsibles", 20, null]])
    );
}

#[tokio::test]
async fn responsible_query_rejects_duplicates_unknowns_null_and_malformed_values_before_workflow() {
    for query in ["limit=0","limit=101","limit=-1","limit=+1","limit=1.0","limit=","limit=null","limit=1&limit=2","after_id=","after_id=null","after_id=invalid","after_id=00000000-0000-0000-0000-000000000000&after_id=00000000-0000-0000-0000-000000000001","status=active","scope=all"] {
        let workflow=Arc::new(Workflow::default());
        let (status,body)=request(workflow.clone(),"GET",&format!("{BASE}/responsibles?{query}"),Some("owner"),None,&[]).await;
        assert_eq!(status,400,"{query}: {body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
    let workflow = Arc::new(Workflow::default());
    let (status, body) = request(
        workflow.clone(),
        "GET",
        "/api/v1/cases/invalid/deadlines/responsibles",
        Some("owner"),
        None,
        &[],
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn responsible_roles_authentication_and_scope_errors_keep_http_status() {
    for (token, failure, expected) in [
        (None, None, 401),
        (Some("client"), None, 403),
        (Some("owner"), Some("case"), 404),
        (Some("owner"), Some("session"), 401),
        (Some("litigator"), None, 200),
        (Some("paralegal"), None, 200),
    ] {
        let workflow = Arc::new(Workflow::default());
        *workflow.failure.lock().unwrap() = failure;
        let (status, body) = request(
            workflow,
            "GET",
            &format!("{BASE}/responsibles"),
            token,
            None,
            &[],
        )
        .await;
        assert_eq!(status, expected, "{body}");
    }
}

#[tokio::test]
async fn responsible_corrupt_workflow_pages_are_masked_as_internal_error() {
    for mutation in 0..9 {
        let mut value = page();
        match mutation {
            0 => value.case_id = CaseId::new(),
            1 => value.responsibles[0].role = Role::Client,
            2 => value.responsibles[0].email = " ".into(),
            3 => value.responsibles.push(value.responsibles[0].clone()),
            4 => value.responsibles.clear(),
            5 => value.next_after_id = None,
            6 => value.next_after_id = Some(id(9)),
            7 => value.has_more = false,
            _ => value.responsibles[0].id = id(0),
        }
        let workflow = Arc::new(Workflow::default());
        *workflow.responsible_page.lock().unwrap() = Some(value);
        let (status, body) = request(
            workflow,
            "GET",
            &format!("{BASE}/responsibles?limit=1&after_id={}", id(0)),
            Some("owner"),
            None,
            &[],
        )
        .await;
        assert_eq!(status, 500, "mutation {mutation}: {body}");
        assert!(!body.to_string().contains("staff@example.test"));
    }
}
