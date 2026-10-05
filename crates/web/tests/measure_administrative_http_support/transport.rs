use super::*;

#[tokio::test]
async fn malformed_closed_objects_references_and_actions_never_call_a_workflow() {
    let original = command_json(&correct().capture.review);
    for fault in 0..25 {
        let mut body = original.clone();
        match fault {
            0 => body["case_id"] = json!("00000000-0000-0000-0000-000000000099"),
            1 => body["operation_id"] = json!("00000000-0000-0000-0000-000000000ABC"),
            2 => body["operation_id"] = json!("000000000000000000000000000001f4"),
            3 => body["target"]["revision"] = json!(0),
            4 => body["target"]["revision"] = json!(4294967296u64),
            5 => body["target"]["revision"] = json!(1.0),
            6 => body["target"]["revision"] = json!("1"),
            7 => body["target"]["capture_digest"] = json!("AB".repeat(32)),
            8 => body["context"]["administration_revision"] = json!(0),
            9 => body["context"]["stage_revision"] = json!(true),
            10 => body["context"]["context_digest"] = json!("ab"),
            11 => body["reason"] = json!(""),
            12 => body["reason"] = json!("x".repeat(1001)),
            13 => body["reason"] = json!("A\tB"),
            14 => body["actor"] = json!({"role":"owner"}),
            15 => body["target"]["head"] = json!(true),
            16 => body["context"] = json!([1, 1, "ab"]),
            17 => body["action"]["kind"] = json!("revoke"),
            18 => body["action"]["values"]["subject"] = json!({}),
            19 => {
                body["action"]["values"]
                    .as_object_mut()
                    .unwrap()
                    .remove("supervision_text");
            }
            20 => {
                body.as_object_mut().unwrap().remove("reason");
            }
            21 => body["action"] = json!(null),
            22 => body["action"]["values"] = json!(["conditions", {}, "supervision"]),
            23 => body["action"]["values"]["conditions"] = json!(100),
            _ => body["action"]["values"]["validity"]["statement"] = json!(null),
        }
        let (status, response) = request(
            MockWrite::new(),
            MockRead::new(),
            "POST",
            &prepare_path(),
            Some(body),
        )
        .await;
        assert_eq!(status, 400, "fault {fault}: {response}");
    }
}

#[tokio::test]
async fn mark_and_replacement_have_closed_variant_specific_shapes() {
    for fault in 0..10 {
        let mut body = if fault < 2 {
            command_json(&marked().capture.review)
        } else {
            command_json(&replacement().capture.review)
        };
        match fault {
            0 => body["action"]["values"] = json!({}),
            1 => body["action"]["replacement_id"] = json!("00000000-0000-0000-0000-000000000010"),
            2 => {
                body["action"].as_object_mut().unwrap().remove("subject");
            }
            3 => body["action"]["subject"] = json!(null),
            4 => body["action"]["subject"]["revision"] = json!(0),
            5 => body["action"]["subject"]["values_digest"] = json!("FF".repeat(32)),
            6 => body["action"]["subject"]["values"] = json!({}),
            7 => body["action"]["supervisor"] = json!({}),
            8 => body["action"]["support"] = json!({}),
            _ => body["action"]["replacement_id"] = json!(false),
        }
        let (status, response) = request(
            MockWrite::new(),
            MockRead::new(),
            "POST",
            &prepare_path(),
            Some(body),
        )
        .await;
        assert_eq!(status, 400, "fault {fault}: {response}");
    }
}

#[tokio::test]
async fn named_json_duplicates_trailing_data_mime_and_body_limit_are_enforced() {
    let encoded = command_json(&correct().capture.review).to_string();
    let duplicate = encoded.replacen('{', "{\"reason\":\"duplicate\",", 1);
    let action = encoded.replace(
        "\"kind\":\"correct\"",
        "\"kind\":\"correct\",\"kind\":\"correct\"",
    );
    let time = encoded.replace(
        "\"precision\":\"unknown\"",
        "\"precision\":\"unknown\",\"precision\":\"unknown\"",
    );
    for (text, mime, expected) in [
        (duplicate, "application/json", 400),
        (action, "application/json", 400),
        (time, "application/json", 400),
        (format!("{encoded} {{}}"), "application/json", 400),
        ("[]".into(), "application/json", 400),
        ("{".into(), "application/json", 400),
        (encoded, "text/plain", 400),
        (" ".repeat(128 * 1024 + 1), "application/json", 413),
    ] {
        let (status, body) = raw(
            (MockWrite::new(), MockRead::new(), MockRecords::new()),
            "POST",
            &prepare_path(),
            "staff-token",
            text,
            mime,
        )
        .await;
        assert_eq!(status, expected, "{body}");
    }
}

#[tokio::test]
async fn submit_requires_both_lowercase_digests_and_no_supplied_receipt() {
    for fault in 0..6 {
        let mut body = submit_json(&correct());
        match fault {
            0 => {
                body.as_object_mut()
                    .unwrap()
                    .remove("expected_submission_digest");
            }
            1 => {
                body.as_object_mut()
                    .unwrap()
                    .remove("expected_review_digest");
            }
            2 => body["expected_submission_digest"] = json!(null),
            3 => body["expected_review_digest"] = json!("AB".repeat(32)),
            4 => body["review"] = json!({"verified":true}),
            _ => body["command"] = json!([]),
        }
        let (status, response) = request(
            MockWrite::new(),
            MockRead::new(),
            "POST",
            &submit_path(),
            Some(body),
        )
        .await;
        assert_eq!(status, 400, "{fault}: {response}");
    }
    let (status, body) = request(
        MockWrite::new(),
        MockRead::new(),
        "POST",
        &format!("{}?force=true", submit_path()),
        Some(submit_json(&correct())),
    )
    .await;
    assert_eq!(status, 400, "{body}");
}
