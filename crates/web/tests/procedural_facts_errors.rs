mod procedural_fact_http_support;
use procedural_fact_http_support::*;
use std::sync::Arc;

#[tokio::test]
async fn case_target_and_source_absence_have_distinct_public_codes() {
    for family in ["resolution", "notification"] {
        for (token, code) in [
            ("case_missing", "case_not_found"),
            ("missing", "procedural_fact_not_found"),
            ("source_missing", "procedural_fact_reference_not_found"),
        ] {
            let workflow = Arc::new(Workflow::default());
            let (status, body) = request(
                workflow.clone(),
                "GET",
                &format!("{}/{ID}/revisions/1", base_url(family)),
                Some(token),
                None,
                &[],
            )
            .await;
            assert_eq!(status, 404, "{body}");
            assert_eq!(body["error"]["code"], code);
            assert!(body.get("values").is_none());
            assert!(body.get("sources").is_none());
            assert_eq!(workflow.calls.lock().unwrap().len(), 1);
        }
    }
}

#[tokio::test]
async fn every_workflow_read_and_mutation_preserves_session_and_permission_denials() {
    for family in ["resolution", "notification"] {
        let base = base_url(family);
        for (token, expected, code) in [
            ("expired", 401, "invalid_session"),
            ("forbidden", 403, "permission_denied"),
        ] {
            for (method, suffix, body) in [
                ("GET", String::new(), None),
                ("GET", format!("/{ID}"), None),
                ("GET", format!("/{ID}/revisions/1"), None),
                ("GET", format!("/{ID}/history"), None),
                (
                    "POST",
                    "/prepare".into(),
                    Some(command_json(family, "record")),
                ),
                (
                    "POST",
                    String::new(),
                    Some(submission(command_json(family, "record"))),
                ),
                (
                    "PUT",
                    format!("/{ID}"),
                    Some(submission(command_json(family, "correct"))),
                ),
                (
                    "POST",
                    format!("/{ID}/withdrawal"),
                    Some(submission(command_json(family, "withdraw"))),
                ),
            ] {
                let workflow = Arc::new(Workflow::default());
                let (status, response) = request(
                    workflow.clone(),
                    method,
                    &format!("{base}{suffix}"),
                    Some(token),
                    body.as_ref().map(ToString::to_string),
                    if body.is_some() {
                        &["application/json"]
                    } else {
                        &[]
                    },
                )
                .await;
                assert_eq!(status, expected, "{method} {suffix}: {response}");
                assert_eq!(response["error"]["code"], code);
                assert_eq!(workflow.calls.lock().unwrap().len(), 1);
            }
        }
    }
}

#[tokio::test]
async fn conflicts_validation_and_internal_failures_are_not_reported_as_success() {
    for family in ["resolution", "notification"] {
        for (token, status, code) in [
            ("closed", 409, "case_closed"),
            ("conflict", 409, "procedural_fact_revision_conflict"),
            ("exhausted", 409, "procedural_fact_revision_exhausted"),
            ("withdrawn", 409, "procedural_fact_already_withdrawn"),
            ("operation", 409, "procedural_fact_operation_conflict"),
            ("mismatch", 409, "procedural_fact_submission_mismatch"),
            ("changed", 409, "procedural_fact_support_changed"),
            (
                "invalid_reference",
                422,
                "procedural_fact_invalid_reference",
            ),
            ("too_large", 422, "procedural_fact_support_too_large"),
            ("format", 422, "procedural_fact_support_format_rejected"),
            ("budget", 422, "procedural_fact_support_validation_limit"),
            ("digest", 422, "procedural_fact_support_digest_mismatch"),
            ("internal", 500, "internal_error"),
        ] {
            let workflow = Arc::new(Workflow::default());
            let (actual, body) = request(
                workflow.clone(),
                "PUT",
                &format!("{}/{ID}", base_url(family)),
                Some(token),
                Some(submission(command_json(family, "correct")).to_string()),
                &["application/json"],
            )
            .await;
            assert_eq!(actual, status, "{token}: {body}");
            assert_eq!(body["error"]["code"], code);
            assert!(!body.to_string().contains("private SQL"));
            assert!(body.get("receipt").is_none());
            assert_eq!(workflow.calls.lock().unwrap().len(), 1);
        }
    }
}
