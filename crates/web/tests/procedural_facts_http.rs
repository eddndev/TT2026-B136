mod procedural_fact_http_support;
use procedural_fact_http_support::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn preparation_preserves_normalized_values_and_unrevised_administration() {
    for family in ["resolution", "notification"] {
        let mut command = command_json(family, "record");
        command["change"]["values"]["summary"] = json!("  Declared\r\nsummary  ");
        let workflow = Arc::new(Workflow::default());
        let (status, body) = request(
            workflow.clone(),
            "POST",
            &format!("{}/prepare", base_url(family)),
            Some("valid"),
            Some(command.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["case_id"], CASE);
        assert_eq!(body["actor_id"], actor().to_string());
        assert_eq!(body["result_revision"], 1);
        assert_eq!(body["command"]["family"], family);
        assert_eq!(body["command"]["id"], ID);
        assert_eq!(body["values"]["summary"], "Declared\nsummary");
        assert_eq!(body["observed_administration"]["kind"], "unrevised");
        assert!(body["observed_administration"].get("revision").is_none());
        assert!(body["observed_administration"]
            .get("values_digest")
            .is_none());
        assert_eq!(
            workflow.commands.lock().unwrap().as_slice(),
            &[typed_command(&command)]
        );
        assert_eq!(workflow.calls.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn record_correction_and_withdrawal_keep_target_action_and_exact_receipt() {
    for family in ["resolution", "notification"] {
        for (action, method, suffix, revision) in [
            ("record", "POST", String::new(), 1),
            ("correct", "PUT", format!("/{ID}"), 2),
            ("withdraw", "POST", format!("/{ID}/withdrawal"), 2),
        ] {
            let command = command_json(family, action);
            let workflow = Arc::new(Workflow::default());
            let (status, body) = request(
                workflow.clone(),
                method,
                &format!("{}{suffix}", base_url(family)),
                Some("valid"),
                Some(submission(command.clone()).to_string()),
                &["application/json"],
            )
            .await;
            assert_eq!(status, 201, "{family} {action}: {body}");
            assert_eq!(body["family"], family);
            assert_eq!(body["id"], ID);
            assert_eq!(body["case_id"], CASE);
            assert_eq!(body["revision"], revision);
            assert_eq!(
                body["status"],
                if action == "withdraw" {
                    "withdrawn"
                } else {
                    "recorded"
                }
            );
            assert_eq!(body["receipt"]["action"], action);
            assert_eq!(body["receipt"]["operation_id"], OPERATION);
            assert_eq!(body["receipt"]["expected_revision"], revision - 1);
            assert_eq!(body["receipt"]["submission_digest"], digest().to_hex());
            if family == "notification" {
                assert_eq!(body["resolution_id"], PARENT);
            }
            let calls = workflow.calls.lock().unwrap();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0]["method"], "submit");
            assert_eq!(calls[0]["expected_submission_digest"], digest().to_hex());
            assert_eq!(
                workflow.commands.lock().unwrap().as_slice(),
                &[typed_command(&command)]
            );
        }
    }
}

#[tokio::test]
async fn roots_exact_revisions_and_lightweight_history_use_separate_queries() {
    for family in ["resolution", "notification"] {
        for suffix in [
            String::new(),
            format!("/{ID}"),
            format!("/{ID}/revisions/7"),
            format!("/{ID}/history"),
        ] {
            let workflow = Arc::new(Workflow::default());
            let (status, body) = request(
                workflow.clone(),
                "GET",
                &format!("{}{suffix}", base_url(family)),
                Some("valid"),
                None,
                &[],
            )
            .await;
            assert_eq!(status, 200, "{family} {suffix}: {body}");
            let calls = workflow.calls.lock().unwrap();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0]["case_id"], CASE);
            if suffix.is_empty() {
                let key = if family == "resolution" {
                    "resolutions"
                } else {
                    "notifications"
                };
                assert_eq!(body[key][0]["id"], ID);
                assert!(body[key][0].get("values").is_none());
                assert!(body[key][0].get("sources").is_none());
                assert_eq!(calls[0]["limit"], 20);
                assert_eq!(body["has_more"], false);
                assert!(body["next_after_id"].is_null());
            } else if suffix.ends_with("history") {
                assert_eq!(body["revisions"][0]["revision"], 1);
                assert!(body["revisions"][0].get("values").is_none());
                assert!(body["revisions"][0].get("sources").is_none());
                assert_eq!(calls[0]["limit"], 10);
                assert_eq!(body["has_more"], false);
                assert!(body["next_before_revision"].is_null());
            } else {
                assert_eq!(
                    body["revision"],
                    if suffix.contains("/revisions/") { 7 } else { 1 }
                );
                assert_eq!(
                    calls[0]["revision"],
                    if suffix.contains("/revisions/") {
                        json!(7)
                    } else {
                        json!(null)
                    }
                );
                if family == "notification" {
                    assert_eq!(body["sources"]["resolution"]["status"], "withdrawn");
                    assert_eq!(body["sources"]["resolution"]["id"], PARENT);
                    assert_eq!(body["sources"]["resolution"]["revision"], 1);
                    assert_eq!(calls[0]["target"]["resolution_id"], PARENT);
                }
            }
        }
    }
}

#[tokio::test]
async fn nil_identifiers_are_valid_and_are_not_replaced_or_dropped() {
    for family in ["resolution", "notification"] {
        let mut command = command_json(family, "record");
        command["id"] = json!(NIL);
        command["operation_id"] = json!(NIL);
        if family == "notification" {
            command["resolution_id"] = json!(NIL);
            command["change"]["values"]["resolution"]["id"] = json!(NIL);
        }
        let path = base_url(family).replace(CASE, NIL).replace(PARENT, NIL);
        let workflow = Arc::new(Workflow::default());
        let (status, body) = request(
            workflow.clone(),
            "POST",
            &format!("{path}/prepare"),
            Some("valid"),
            Some(command.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["case_id"], NIL);
        assert_eq!(body["command"]["id"], NIL);
        assert_eq!(body["command"]["operation_id"], NIL);
        assert_eq!(workflow.calls.lock().unwrap()[0]["case_id"], NIL);
        assert_eq!(
            workflow.commands.lock().unwrap().as_slice(),
            &[typed_command(&command)]
        );
    }
}

#[tokio::test]
async fn mismatched_path_identity_family_parent_and_action_do_not_reach_workflow() {
    for family in ["resolution", "notification"] {
        let own = base_url(family);
        let other = base_url(if family == "resolution" {
            "notification"
        } else {
            "resolution"
        });
        let mut cases = vec![
            (
                "POST",
                format!("{other}/prepare"),
                command_json(family, "record"),
            ),
            (
                "PUT",
                format!("{own}/{NIL}"),
                submission(command_json(family, "correct")),
            ),
            (
                "PUT",
                format!("{own}/{ID}"),
                submission(command_json(family, "record")),
            ),
            (
                "POST",
                own.clone(),
                submission(command_json(family, "correct")),
            ),
            (
                "POST",
                format!("{own}/{ID}/withdrawal"),
                submission(command_json(family, "correct")),
            ),
        ];
        if family == "notification" {
            cases.push((
                "POST",
                format!("{}/prepare", own.replace(PARENT, NIL)),
                command_json(family, "record"),
            ));
            let mut c = command_json(family, "record");
            c["change"]["values"]["resolution"]["id"] = json!(NIL);
            cases.push(("POST", format!("{own}/prepare"), c));
        }
        for (method, path, payload) in cases {
            let workflow = Arc::new(Workflow::default());
            let (status, body) = request(
                workflow.clone(),
                method,
                &path,
                Some("valid"),
                Some(payload.to_string()),
                &["application/json"],
            )
            .await;
            assert!((400..500).contains(&status), "{method} {path}: {body}");
            assert!(workflow.calls.lock().unwrap().is_empty(), "{method} {path}");
        }
    }
}
