use super::*;

#[tokio::test]
async fn closed_command_objects_and_canonical_scalars_reject_before_workflow() {
    let encoded = command(g1().command());
    for fault in 0..24 {
        let mut input = encoded.clone();
        match fault {
            0 => input["case_id"] = json!("00000000-0000-0000-0000-000000000099"),
            1 => input["operation_id"] = json!("00000000000000000000000000000064"),
            2 => input["decision_id"] = json!("00000000-0000-0000-0000-00000000006E"),
            3 => input["unexpected"] = json!(true),
            4 => {
                input.as_object_mut().unwrap().remove("anchor");
            }
            5 => input["context"]["administration_revision"] = json!(0),
            6 => input["context"]["stage_revision"] = json!(1.5),
            7 => input["context"]["context_digest"] = json!("AB".repeat(32)),
            8 => input["values"]["support"]["version"] = json!(0),
            9 => input["values"]["support"]["digest"] = json!("00"),
            10 => input["values"]["declared_at"]["year"] = json!(2020),
            11 => {
                input["values"]["declared_at"]
                    .as_object_mut()
                    .unwrap()
                    .remove("reason");
            }
            12 => {
                input["values"]["declared_at"] =
                    json!({"precision":"date","year":2020,"month":1,"day":2})
            }
            13 => {
                input["values"]["declared_at"] = json!({"precision":"date","year":2020,"month":1,"day":2,"offset_seconds":null,"reason":"unexpected"})
            }
            14 => {
                input["values"]["declared_at"] = json!({"precision":"minute","year":2020,"month":1,"day":2,"hour":12,"minute":0,"offset_seconds":1})
            }
            15 => {
                input["outcome"]["effects"][0]["proposal"]["values"]["kind"] =
                    json!("automatic_detention")
            }
            16 => {
                input["outcome"]["effects"][0]["proposal"]["values"]["validity"]
                    .as_object_mut()
                    .unwrap()
                    .remove("end");
            }
            17 => {
                input["outcome"]["effects"][0]["proposal"]["values"]["supervision"]["verified"] =
                    json!(true)
            }
            18 => {
                input["outcome"]["effects"][0]["proposal"]["values"]["subject"]["revision"] =
                    json!(0)
            }
            19 => input["outcome"]["effects"][0]["proposal"] = json!(["id", "values"]),
            20 => input["outcome"]["effects"][0]["previous"] = json!({}),
            21 => {
                input["anchor"] = json!({"kind":"initial","hearing_id":"00000000-0000-0000-0000-000000000079","revision":1,"values_digest":"00".repeat(32),"submission_digest":"11".repeat(32),"capture_digest":"22".repeat(32)})
            }
            22 => {
                input["anchor"] = json!({"kind":"precautionary","hearing_id":"00000000-0000-0000-0000-000000000028","revision":1,"capture_digest":"22".repeat(32),"values_digest":"00".repeat(32)})
            }
            _ => {
                input["outcome"] =
                    json!({"kind":"no_measure_change","statement":"none","effects":[]})
            }
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
async fn duplicate_trailing_mime_auth_and_confirmation_errors_never_call_workflow() {
    let encoded = command(g1().command()).to_string();
    let duplicate = encoded.replacen('{', "{\"operation_id\":\"duplicate\",", 1);
    let nested = encoded.replace(
        "\"action\":\"impose\"",
        "\"action\":\"impose\",\"action\":\"impose\"",
    );
    for (path, token, body, mime, expected) in [
        ("prepare", "", encoded.clone(), "application/json", 401),
        (
            "prepare?limit=1",
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
            " ".repeat(4 * 1024 * 1024 + 1),
            "application/json",
            413,
        ),
    ] {
        let (status, value) = raw(
            (MockWrite::new(), MockRead::new()),
            "POST",
            &format!("{}/{path}", base()),
            token,
            body,
            mime,
        )
        .await;
        assert_eq!(status, expected, "{value}");
    }
    for field in ["expected_submission_digest", "expected_review_digest"] {
        for absent in [true, false] {
            let mut input = submission(&g2());
            if absent {
                input.as_object_mut().unwrap().remove(field);
            } else {
                input[field] = json!("AA".repeat(32));
            }
            let (status, value) = request(
                MockWrite::new(),
                MockRead::new(),
                "POST",
                &format!("{}/submit", base()),
                Some(input),
            )
            .await;
            assert_eq!(status, 400, "{field}: {value}");
        }
    }
}

#[tokio::test]
async fn substitutions_enforce_the_joint_32_identity_bound_and_disjoint_effects() {
    let original = command(g1().command());
    let proposal = original["outcome"]["effects"][0]["proposal"].clone();
    let prior = reference(crate::measure_decision_fixtures::reference(
        &crate::measure_decision_fixtures::Fixture::single()
            .capture()
            .measures[0],
    ));
    for fault in 0..5 {
        let mut input = original.clone();
        let successors: Vec<_> = (0..32)
            .map(|n| {
                let mut p = proposal.clone();
                p["id"] = json!(uuid::Uuid::from_u128(1000 + n).to_string());
                p
            })
            .collect();
        input["outcome"] = match fault {
            0 => {
                json!({"kind":"changes","effects":[{"action":"substitute","predecessors":[prior.clone()],"successors":successors}]})
            }
            1 => {
                json!({"kind":"changes","effects":vec![json!({"action":"impose","proposal":proposal});33]})
            }
            2 => {
                json!({"kind":"changes","effects":[{"action":"confirm","previous":prior.clone()},{"action":"revoke","previous":prior.clone()}]})
            }
            3 => {
                json!({"kind":"changes","effects":[{"action":"substitute","predecessors":[],"successors":[proposal.clone()]}]})
            }
            _ => {
                json!({"kind":"changes","effects":[{"action":"substitute","predecessors":[prior.clone()],"successors":[]}]})
            }
        };
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
