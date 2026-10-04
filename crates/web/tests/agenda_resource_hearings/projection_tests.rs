use super::*;

#[tokio::test]
async fn resource_hearing_projection_has_its_own_capture_without_stage_or_status() {
    let (workflow, router) = setup(own_page());
    let (status, body) = request(router, &format!("{RANGE}&kind=resource_hearing"), true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["kind"], "resource_hearing");
    let row = &body["items"][0];
    assert_eq!(row["kind"], "resource_hearing");
    assert_eq!(row["case_title"], "Authorized resource case");
    assert_eq!(row["case_reference"], "RESOURCE-1");
    assert_eq!(row["case_status"], "closed");
    assert_eq!(
        row["at"],
        json!({
            "unix_seconds":1767225601i64,"nanosecond":0,"offset_seconds":0,
        })
    );
    assert_eq!(
        row["resource_hearing"],
        json!({
            "case_id":Uuid::from_u128(1).to_string(),
            "resource_id":Uuid::from_u128(3).to_string(),
            "id":Uuid::from_u128(2).to_string(), "revision":1,
            "kind":"appeal_arguments", "modality":"in_person", "participant_count":2,
            "scheduled_at":"2025-12-31T18:00:01-06:00",
            "association_id":Uuid::from_u128(4).to_string(),
            "capture_digest":"07".repeat(32),
        })
    );
    assert!(row.get("hearing").is_none());
    assert!(row.get("stage").is_none());
    assert!(row.get("status").is_none());
    let calls = workflow.calls.lock().unwrap();
    assert_eq!(calls[0].0, "agenda-session");
    assert_eq!(calls[0].1.kind(), AgendaKind::ResourceHearing);
}

#[tokio::test]
async fn own_hearing_filter_preserves_status_rules_and_ordinary_family_is_exclusive() {
    for kind in ["all", "resource_hearing"] {
        for status in ["scheduled", "all"] {
            let (workflow, router) = setup(own_page());
            let query = format!("{RANGE}&kind={kind}&hearing_status={status}");
            let (code, body) = request(router, &query, true).await;
            assert_eq!(code, StatusCode::OK, "{query}: {body}");
            assert_eq!(body["hearing_status"], status);
            assert_eq!(workflow.calls.lock().unwrap().len(), 1);
        }
    }
    let (workflow, router) = setup(empty_page());
    let query = format!("{RANGE}&kind=resource_hearing&hearing_status=cancelled");
    assert_eq!(
        request(router, &query, true).await.0,
        StatusCode::BAD_REQUEST
    );
    assert!(workflow.calls.lock().unwrap().is_empty());
    for suffix in [
        "&kind=hearing",
        "&kind=deadline",
        "&hearing_status=cancelled",
    ] {
        let (_, router) = setup(own_page());
        let (code, body) = request(router, &format!("{RANGE}{suffix}"), true).await;
        assert_eq!(code, StatusCode::INTERNAL_SERVER_ERROR, "{suffix}");
        assert!(!body.to_string().contains("Authorized resource case"));
    }
}

#[tokio::test]
async fn three_families_can_share_uuid_and_instant_without_colliding() {
    let value = mixed_page();
    assert_eq!(
        value.items[0].key().unwrap().id(),
        value.items[2].key().unwrap().id()
    );
    let (_, router) = setup(value.clone());
    let range = "from=2026-01-06T00:00:00Z&until=2026-01-07T00:00:00Z";
    let (status, body) = request(router, range, true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let rows = body["items"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .map(|v| v["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["hearing", "deadline", "resource_hearing"]
    );
    assert_eq!(rows[0]["at"], rows[1]["at"]);
    assert_eq!(rows[1]["at"], rows[2]["at"]);
    let mut cancelled = value;
    cancelled.items.pop();
    let AgendaItem::Hearing(ordinary) = &mut cancelled.items[0] else {
        unreachable!()
    };
    ordinary.status = HearingStatus::Cancelled;
    let (_, router) = setup(cancelled);
    let (status, body) = request(router, &format!("{range}&hearing_status=cancelled"), true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn corrupt_resource_hearing_scope_and_shape_do_not_disclose_payload() {
    for fault in 0..6 {
        let mut value = own_page();
        let AgendaItem::ResourceHearing { case, hearing } = &mut value.items[0] else {
            unreachable!()
        };
        match fault {
            0 => case.case_id = CaseId::from_uuid(Uuid::from_u128(99)),
            1 => case.title = " padded".into(),
            2 => hearing.participant_count = 33,
            3 => hearing.revision = ResourceHearingRevision::new(2).unwrap(),
            4 => hearing.scheduled_at = HearingTime::new(at(1767139200)).unwrap(),
            _ => value.items.push(resource_hearing()),
        }
        let (_, router) = setup(value);
        let (status, body) = request(router, RANGE, true).await;
        assert_eq!(
            status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "fault {fault}: {body}"
        );
        assert!(!body.to_string().contains("Authorized resource case"));
        assert!(!body.to_string().contains("appeal_arguments"));
    }
}

#[tokio::test]
async fn resource_hearing_requests_preserve_authentication_and_opaque_workflow_errors() {
    struct Failed(u8);
    impl AgendaWorkflow for Failed {
        fn list(&self, _: &str, _: AgendaQuery) -> Result<AgendaPage, ApplicationError> {
            Err(match self.0 {
                0 => ApplicationError::InvalidSession,
                1 => ApplicationError::PermissionDenied,
                _ => ApplicationError::Port("secret resource capture".into()),
            })
        }
    }
    let query = format!("{RANGE}&kind=resource_hearing");
    let (workflow, router) = setup(own_page());
    assert_eq!(
        request(router, &query, false).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert!(workflow.calls.lock().unwrap().is_empty());
    for (fault, expected) in [
        (0, StatusCode::UNAUTHORIZED),
        (1, StatusCode::FORBIDDEN),
        (2, StatusCode::INTERNAL_SERVER_ERROR),
    ] {
        let (status, body) =
            request(web::agenda_router(Arc::new(Failed(fault))), &query, true).await;
        assert_eq!(status, expected, "{body}");
        assert!(body.get("items").is_none());
        assert!(!body.to_string().contains("secret resource capture"));
    }
}
