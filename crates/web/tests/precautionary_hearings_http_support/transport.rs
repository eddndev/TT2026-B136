use super::*;

#[tokio::test]
async fn malformed_or_overbound_command_shapes_never_reach_the_workflow() {
    for fault in 0..20 {
        let mut input = body();
        match fault {
            0 => input["case_id"] = json!("00000000-0000-0000-0000-000000000002"),
            1 => input["operation_id"] = json!("0000000000000000000000000000001e"),
            2 => input["operation_id"] = json!("00000000-0000-0000-0000-00000000001E"),
            3 => input["unexpected"] = json!(true),
            4 => input["change"]["expected_revision"] = json!(1),
            5 => input["change"]["context"]["administration_revision"] = json!(0),
            6 => input["change"]["context"]["context_digest"] = json!("AB".repeat(32)),
            7 => input["change"]["values"]["scheduled_at"] = json!("2027-01-01T00:00:00.1Z"),
            8 => input["change"]["values"]["scheduled_at"] = json!("2027-01-01T00:00:00-00:00"),
            9 => {
                input["change"]["values"]
                    .as_object_mut()
                    .unwrap()
                    .remove("note");
            }
            10 => input["change"]["values"]["purpose"] = json!("review"),
            11 => {
                input["change"]["values"]["review_targets"] = json!([{"id":"00000000-0000-0000-0000-000000000046","revision":1,"capture_digest":"09".repeat(32)}])
            }
            12 => {
                input["change"]["values"]["participants"] = json!(vec![
                    json!({"participant_id":"00000000-0000-0000-0000-00000000000a","revision":1});
                    33
                ])
            }
            13 => input["change"]["values"]["participants"][0]["revision"] = json!(0),
            14 => input["change"]["values"]["scheduling_basis"]["support"]["version"] = json!(0),
            15 => input["change"]["values"]["scheduling_basis"]["locator"] = json!(""),
            16 => input["change"]["values"]["scheduling_basis"]["support"]["digest"] = json!("00"),
            17 => input["change"]["values"]["participants"][0] = json!(["id", 1]),
            18 => input["change"]["values"]["modality"] = json!("telephone"),
            _ => input["change"]["values"]["scheduling_basis"]["support"]["verified"] = json!(true),
        }
        let (status, value) = request(
            MockContext::new(),
            MockWrite::new(),
            MockRead::new(),
            "POST",
            &format!("{}/prepare", base()),
            Some(input),
        )
        .await;
        assert_eq!(status, 400, "fault {fault}: {value}");
    }
    for field in ["context", "values"] {
        let mut input = command(&cancelled().capture.review.command);
        input["change"][field] = json!({});
        let (status, value) = request(
            MockContext::new(),
            MockWrite::new(),
            MockRead::new(),
            "POST",
            &format!("{}/prepare", base()),
            Some(input),
        )
        .await;
        assert_eq!(status, 400, "cancel {field}: {value}");
    }
}

#[tokio::test]
async fn strict_entity_authentication_and_two_digest_confirmation_are_mandatory() {
    let encoded = body().to_string();
    let duplicate = encoded.replacen('{', "{\"operation_id\":\"duplicate\",", 1);
    let nested = encoded.replace(
        "\"action\":\"schedule\"",
        "\"action\":\"schedule\",\"action\":\"schedule\"",
    );
    for (suffix, token, body, mime, expected) in [
        ("prepare", "", encoded.clone(), "application/json", 401),
        (
            "prepare?x=1",
            "staff-token",
            encoded.clone(),
            "application/json",
            400,
        ),
        ("prepare", "staff-token", encoded.clone(), "text/plain", 400),
        ("prepare", "staff-token", duplicate, "application/json", 400),
        ("prepare", "staff-token", nested, "application/json", 400),
        (
            "prepare",
            "staff-token",
            format!("{encoded} {{}}"),
            "application/json",
            400,
        ),
        (
            "prepare",
            "staff-token",
            "[]".into(),
            "application/json",
            400,
        ),
        (
            "prepare",
            "staff-token",
            " ".repeat(128 * 1024 + 1),
            "application/json",
            413,
        ),
    ] {
        let (status, value) = raw(
            (MockContext::new(), MockWrite::new(), MockRead::new()),
            "POST",
            &format!("{}/{suffix}", base()),
            token,
            body,
            mime,
        )
        .await;
        assert_eq!(status, expected, "{value}");
    }
    for fault in 0..5 {
        let mut input = submission(&initial());
        match fault {
            0 => {
                input
                    .as_object_mut()
                    .unwrap()
                    .remove("expected_submission_digest");
            }
            1 => {
                input
                    .as_object_mut()
                    .unwrap()
                    .remove("expected_review_digest");
            }
            2 => input["expected_review_digest"] = json!("AB".repeat(32)),
            3 => input["expected_submission_digest"] = json!(null),
            _ => input["review"] = json!({"verified":true}),
        }
        let (status, value) = request(
            MockContext::new(),
            MockWrite::new(),
            MockRead::new(),
            "POST",
            &format!("{}/submit", base()),
            Some(input),
        )
        .await;
        assert_eq!(status, 400, "confirmation {fault}: {value}");
    }
}
