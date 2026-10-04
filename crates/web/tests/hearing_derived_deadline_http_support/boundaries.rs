use super::*;

#[tokio::test]
async fn queries_content_types_and_non_object_envelopes_are_rejected() {
    for suffix in ["/prepare?", "/prepare?unexpected=1", "/submit?x=1"] {
        let workflow = Arc::new(Workflow::default());
        let input = if suffix.starts_with("/submit") {
            submission()
        } else {
            command()
        };
        let (status, body) = request(
            workflow.clone(),
            suffix,
            Some("owner"),
            input.to_string(),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 400, "{suffix}: {body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
    for types in [
        vec![],
        vec!["text/plain"],
        vec!["application/json", "application/json"],
    ] {
        let workflow = Arc::new(Workflow::default());
        let (status, body) = request(
            workflow.clone(),
            "/prepare",
            Some("owner"),
            command().to_string(),
            &types,
        )
        .await;
        assert_eq!(status, 400, "{body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
    for input in [json!([]), json!(null), json!({})] {
        let workflow = Arc::new(Workflow::default());
        let (status, body) = prepare(workflow.clone(), input).await;
        assert_eq!(status, 400, "{body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn unknown_duplicate_and_array_fields_are_strict_at_nested_boundaries() {
    for pointer in [
        "",
        "/result",
        "/result/change",
        "/result/change/values/event_time",
        "/deadline",
        "/deadline/change",
        "/deadline/change/definition/input/selection/source/value",
        "/deadline/change/definition/input/qualification",
    ] {
        for array in [false, true] {
            let mut input = command();
            let object = input.pointer_mut(pointer).unwrap();
            if array {
                *object = json!([]);
            } else {
                object
                    .as_object_mut()
                    .unwrap()
                    .insert("unexpected".into(), json!(true));
            }
            let workflow = Arc::new(Workflow::default());
            let (status, body) = prepare(workflow.clone(), input).await;
            assert_eq!(status, 400, "{pointer}: {body}");
            assert!(workflow.calls.lock().unwrap().is_empty());
        }
    }
    let original = command().to_string();
    for key in [
        "case_id",
        "operation_id",
        "action",
        "precision",
        "revision",
        "statement",
    ] {
        let raw = original.replacen(
            &format!("\"{key}\":"),
            &format!("\"{key}\":null,\"{key}\":"),
            1,
        );
        let workflow = Arc::new(Workflow::default());
        let (status, body) = request(
            workflow.clone(),
            "/prepare",
            Some("owner"),
            raw,
            &["application/json"],
        )
        .await;
        assert_eq!(status, 400, "{key}: {body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn submit_requires_exact_digest_shape_and_preserves_the_parsed_digest() {
    for value in [
        json!("f".repeat(63)),
        json!("g".repeat(64)),
        json!(null),
        json!([]),
    ] {
        let mut input = submission();
        input["expected_review_digest"] = value;
        let workflow = Arc::new(Workflow::default());
        let (status, body) = request(
            workflow.clone(),
            "/submit",
            Some("owner"),
            input.to_string(),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 400, "{body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
    let workflow = Arc::new(Workflow::default());
    let (status, body) = request(
        workflow.clone(),
        "/submit",
        Some("owner"),
        submission().to_string(),
        &["application/json"],
    )
    .await;
    assert_eq!(status, 422, "{body}");
    assert_eq!(
        workflow.calls.lock().unwrap()[0].3,
        Some(deadline_http_support::digest())
    );
}

#[tokio::test]
async fn one_mebibyte_limit_counts_whitespace_without_truncating_the_command() {
    let raw = command().to_string();
    let padded = format!("{raw}{}", " ".repeat(1024 * 1024 - raw.len()));
    for (body, expected, count) in [(padded.clone(), 422, 1), (format!("{padded} "), 413, 0)] {
        let workflow = Arc::new(Workflow::default());
        let (status, response) = request(
            workflow.clone(),
            "/prepare",
            Some("owner"),
            body,
            &["application/json"],
        )
        .await;
        assert_eq!(status, expected, "{response}");
        assert_eq!(workflow.calls.lock().unwrap().len(), count);
    }
}

#[tokio::test]
async fn submission_envelope_rejects_extras_duplicates_and_positional_commands() {
    let original = submission();
    let mut extra = original.clone();
    extra["unexpected"] = json!(true);
    let mut positional = original.clone();
    positional["command"] = json!([CASE, command()["result"], command()["deadline"]]);
    for raw in [
        extra.to_string(),
        positional.to_string(),
        original.to_string().replace(
            "\"expected_review_digest\":",
            "\"expected_review_digest\":null,\"expected_review_digest\":",
        ),
        format!("{} trailing", original),
    ] {
        let workflow = Arc::new(Workflow::default());
        let (status, body) = request(
            workflow.clone(),
            "/submit",
            Some("owner"),
            raw,
            &["application/json"],
        )
        .await;
        assert_eq!(status, 400, "{body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn invalid_domain_bounds_are_rejected_before_workflow() {
    for (pointer, expected) in [
        ("/result/change/anchor_revision", 422),
        ("/deadline/change/definition/profile/revision", 422),
        ("/deadline/change/definition/input/ordered_quantity", 400),
    ] {
        let mut input = command();
        *input.pointer_mut(pointer).unwrap() = json!(0);
        let workflow = Arc::new(Workflow::default());
        let (status, body) = prepare(workflow.clone(), input).await;
        assert_eq!(status, expected, "{pointer}: {body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn nil_agreement_identity_and_unknown_qualification_are_preserved() {
    let mut input = command();
    input["result"]["change"]["values"]["agreements"] =
        json!([{"id":deadline_http_support::ID,"text":"Declared agreement"}]);
    let selection = &mut input["deadline"]["change"]["definition"]["input"]["selection"];
    selection["source"]["value"]["agreement_id"] = json!(deadline_http_support::ID);
    selection["qualification"] = json!({"purpose":"hearing_end","at":{"precision":"unknown"},"statement":"End time not established","locator":"Declared agreement"});
    let workflow = Arc::new(Workflow::default());
    let (status, body) = prepare(workflow.clone(), input).await;
    assert_eq!(status, 422, "{body}");
    let calls = workflow.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let (deadline, _) = calls[0].2.deadline.clone().into_parts();
    let DeadlineChange::Register { definition } = deadline.change else {
        panic!("register expected")
    };
    let selection = definition.input.selection;
    let FactDeclaration::Known(TriggerSourceRef::HearingResult(source)) = selection.source else {
        panic!("hearing expected")
    };
    assert_eq!(source.agreement_id.unwrap().as_uuid(), uuid::Uuid::nil());
    assert_eq!(
        selection.qualification.unwrap().at,
        domain::procedural_time::DeclaredProceduralTime::unknown()
    );
}
