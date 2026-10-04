use super::*;

#[tokio::test]
async fn preparation_preserves_exact_command_and_historical_resource_selection() {
    let mut write = MockWrite::new();
    write
        .expect_prepare()
        .times(1)
        .return_once(|token, c, r, input| {
            assert_eq!((token, c, r), ("owner", case(), resource()));
            assert_eq!(input, command());
            Ok(draft())
        });
    let (status, value) = request(
        write,
        MockRead::new(),
        "POST",
        &format!("{}/prepare", base()),
        Some(body()),
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["command"], body());
    assert_eq!(value["submission_digest"], digest().to_hex());
    assert_eq!(value["resource"]["revision"], 2);
    assert_eq!(value["observed_resource_head"]["revision"], 5);
    assert!(value.get("scheduling_context").is_none());
}

#[tokio::test]
async fn explicit_resubmission_returns_the_original_creation_and_initial_association() {
    let mut prior = None;
    for _ in 0..2 {
        let mut write = MockWrite::new();
        write
            .expect_submit()
            .times(1)
            .return_once(|token, c, r, input, expected| {
                assert_eq!((token, c, r), ("owner", case(), resource()));
                assert_eq!(input, command());
                assert_eq!(expected, digest());
                Ok(creation())
            });
        let (status, value) = request(
            write,
            MockRead::new(),
            "POST",
            &format!("{}/submit", base()),
            Some(submission()),
        )
        .await;
        assert_eq!(status, 201, "{value}");
        assert_eq!(value["hearing"]["id"], hearing_id().to_string());
        assert_eq!(value["hearing"]["values"]["kind"], "written_revocation");
        assert_eq!(
            value["association"]["selection"]["target"]["kind"],
            "resource_hearing"
        );
        assert_eq!(
            value["origin"]["operation_id"],
            command().operation_id.to_string()
        );
        assert_eq!(value["submission_digest"], digest().to_hex());
        if let Some(previous) = prior {
            assert_eq!(value, previous);
        }
        prior = Some(value);
    }
}

#[tokio::test]
async fn invalid_commands_never_reach_the_workflow() {
    for fault in 0..16 {
        let mut input = body();
        match fault {
            0 => input["case_id"] = json!(resource().to_string()),
            1 => input["resource_id"] = json!(case().to_string()),
            2 => input["resource"]["id"] = json!(case().to_string()),
            3 => {
                input.as_object_mut().unwrap().remove("act");
            }
            4 => input["unknown"] = json!(true),
            5 => input["resource"]["revision"] = json!(0),
            6 => input["expected_resource_revision"] = json!(0),
            7 => {
                input["operation_id"] = json!(command().operation_id.as_uuid().simple().to_string())
            }
            8 => input["values"]["recorded_stage"] = json!(1),
            9 => input["values"]["kind"] = json!("intermediate"),
            10 => input["values"]["scheduled_at"] = json!("1970-01-03T00:00:00.1Z"),
            11 => {
                input["values"]["participants"] =
                    json!([{"participant_id":case().to_string(),"revision":0}])
            }
            12 => input["values"]["scheduling_basis"]["support"]["digest"] = json!("AB".repeat(32)),
            13 => input["values"]["scheduling_basis"]["support"]["version"] = json!(0),
            14 => {
                input["act"] = json!({"id":case().to_string(),"revision":1,"resource_revision":0,"capture_digest":digest().to_hex()})
            }
            _ => input["values"]["scheduling_basis"]["unexpected"] = json!(true),
        }
        let (status, value) = request(
            MockWrite::new(),
            MockRead::new(),
            "POST",
            &format!("{}/prepare", base()),
            Some(input),
        )
        .await;
        assert_eq!(status, 400, "fault {fault}: {value}");
    }
}

#[tokio::test]
async fn request_transport_rejects_missing_auth_duplicates_queries_and_oversize_json() {
    let encoded = body().to_string();
    let duplicate = encoded.replacen('{', "{\"case_id\":\"duplicate\",", 1);
    for (suffix, token, input, content_type, expected) in [
        ("prepare", "", encoded.clone(), "application/json", 401),
        (
            "prepare?extra=1",
            "owner",
            encoded.clone(),
            "application/json",
            400,
        ),
        ("prepare", "owner", encoded, "text/plain", 400),
        ("prepare", "owner", duplicate, "application/json", 400),
        (
            "prepare",
            "owner",
            " ".repeat(65537),
            "application/json",
            413,
        ),
        ("prepare", "owner", "[]".into(), "application/json", 400),
    ] {
        let (status, value) = raw(
            MockWrite::new(),
            MockRead::new(),
            "POST",
            &format!("{}/{suffix}", base()),
            token,
            input,
            content_type,
        )
        .await;
        assert_eq!(status, expected, "{value}");
    }
    for value in [
        json!({"command":body(),"expected_submission_digest":"AB".repeat(32)}),
        json!({"command":body(),"expected_submission_digest":digest().to_hex(),"retry":true}),
    ] {
        let (status, result) = request(
            MockWrite::new(),
            MockRead::new(),
            "POST",
            &format!("{}/submit", base()),
            Some(value),
        )
        .await;
        assert_eq!(status, 400, "{result}");
    }
}
