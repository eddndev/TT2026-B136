use super::*;

#[tokio::test]
async fn estimate_and_email_are_explicitly_unknown_without_stored_metadata() {
    let (_, router) = setup(false, None);
    let (status, headers, bytes) = send(router, "GET", &format!("/{ID}"), None, true).await;
    assert_eq!(status, StatusCode::OK);
    private(&headers);
    let value = body(&bytes);
    assert_eq!(value.get("estimated_total_ms"), Some(&json!(null)));
    assert_eq!(value.get("ready_email"), Some(&json!(null)));
}

#[tokio::test]
async fn estimate_is_shared_by_request_list_detail_and_notice_receipts() {
    let (workflow, router) = setup(true, None);
    let mut value = report(true);
    value.estimate = Some(CaseReportEstimate::new(65_000).unwrap());
    value.ready_email = Some(CaseReportEmailState::Pending);
    *workflow.detail.lock().unwrap() = Some(value);
    for (method, path, payload, is_list) in [
        ("POST", String::new(), Some(request_body()), false),
        ("GET", String::new(), None, true),
        ("GET", format!("/{ID}"), None, false),
        ("POST", format!("/{ID}/notice-read"), Some(json!({})), false),
    ] {
        let (status, headers, bytes) = send(router.clone(), method, &path, payload, true).await;
        assert!(status.is_success(), "{method} {path}: {status}");
        private(&headers);
        let response = body(&bytes);
        let detail = if is_list {
            &response["reports"][0]
        } else {
            &response
        };
        assert_eq!(detail.get("estimated_total_ms"), Some(&json!(65_000)));
        assert_eq!(detail.get("ready_email"), Some(&json!("pending")));
        for private in [
            "requester",
            "email",
            "account_revision",
            "auth_generation",
            "lease_until",
        ] {
            assert!(detail.get(private).is_none(), "private field: {private}");
        }
    }
}

#[tokio::test]
async fn email_transport_exposes_provider_outcomes_without_delivery_claims() {
    for (state, expected) in [
        (CaseReportEmailState::Pending, "pending"),
        (CaseReportEmailState::Accepted, "accepted"),
        (CaseReportEmailState::Rejected, "rejected"),
        (CaseReportEmailState::Uncertain, "uncertain"),
        (CaseReportEmailState::AccessRevoked, "access_revoked"),
    ] {
        let (workflow, router) = setup(true, None);
        let mut value = report(true);
        value.estimate = Some(CaseReportEstimate::new(2_001).unwrap());
        value.ready_email = Some(state);
        *workflow.detail.lock().unwrap() = Some(value);
        let (status, _, bytes) = send(router, "GET", &format!("/{ID}"), None, true).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body(&bytes).get("ready_email"), Some(&json!(expected)));
    }
}

#[tokio::test]
async fn estimated_milliseconds_are_exact_safe_json_integers() {
    for total_ms in [1, 2_000, 9_007_199_254_740_991] {
        let (workflow, router) = setup(false, None);
        let mut value = report(false);
        value.estimate = Some(CaseReportEstimate::new(total_ms).unwrap());
        *workflow.detail.lock().unwrap() = Some(value);
        let (status, _, bytes) = send(router, "GET", &format!("/{ID}"), None, true).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body(&bytes).get("estimated_total_ms"),
            Some(&json!(total_ms))
        );
        assert_eq!(body(&bytes).get("ready_email"), Some(&json!(null)));
    }
    let (workflow, router) = setup(false, None);
    let mut value = report(false);
    value.estimate = Some(CaseReportEstimate::new(9_007_199_254_740_992).unwrap());
    *workflow.detail.lock().unwrap() = Some(value);
    let (status, _, bytes) = send(router, "GET", &format!("/{ID}"), None, true).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body(&bytes).get("estimated_total_ms").is_none());
}
