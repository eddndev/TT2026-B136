use crate::member_invitations_support::*;
use axum::{body::Body, http::StatusCode};
use serde_json::json;

#[tokio::test]
async fn recipient_bodies_reject_ambiguous_tokens_unknown_fields_queries_and_media_types() {
    let fixture = Harness::ready();
    for action in ["prepare", "finish", "receipt"] {
        let path = format!("{PUBLIC}/{action}");
        let mut valid = recipient();
        if action == "finish" {
            valid["code"] = json!(CODE);
        }
        for malformed in [
            json!(null),
            json!([]),
            json!({}),
            {
                let mut v = valid.clone();
                v["unexpected"] = json!(true);
                v
            },
            {
                let mut v = valid.clone();
                v["operation_id"] = json!(uuid::Uuid::nil());
                v
            },
        ] {
            assert_eq!(
                call(&fixture.router, "POST", &path, None, Some(malformed))
                    .await
                    .status,
                StatusCode::BAD_REQUEST
            );
        }
        for invalid in [
            "".into(),
            "x".repeat(43),
            format!("{}=", token()),
            "A".repeat(42),
            format!("{}B", "A".repeat(42)),
            "-".repeat(44),
        ] {
            let mut body = valid.clone();
            body["token"] = json!(invalid);
            call(&fixture.router, "POST", &path, None, Some(body))
                .await
                .error(StatusCode::BAD_REQUEST, "invitation_rejected");
        }
        assert_eq!(
            call(
                &fixture.router,
                "POST",
                &format!("{path}?token=secret"),
                None,
                Some(valid.clone())
            )
            .await
            .status,
            StatusCode::BAD_REQUEST
        );
        let duplicate = valid
            .to_string()
            .replacen('{', "{\"token\":\"duplicate\",", 1);
        assert_eq!(
            raw(
                &fixture.router,
                "POST",
                &path,
                Body::from(duplicate),
                &[("content-type", "application/json")]
            )
            .await
            .status,
            StatusCode::BAD_REQUEST
        );
        for headers in [
            vec![],
            vec![("content-type", "text/plain")],
            vec![
                ("content-type", "application/json"),
                ("content-type", "application/json"),
            ],
        ] {
            assert_eq!(
                raw(
                    &fixture.router,
                    "POST",
                    &path,
                    Body::from(valid.to_string()),
                    &headers
                )
                .await
                .status,
                StatusCode::UNSUPPORTED_MEDIA_TYPE
            );
        }
        let large = format!("{{\"padding\":\"{}\"}}", "x".repeat(16 * 1024));
        let reply = raw(
            &fixture.router,
            "POST",
            &path,
            Body::from(large),
            &[("content-type", "application/json")],
        )
        .await;
        reply.metadata();
        assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    }
    assert!(fixture.workflow.calls().is_empty());
}

#[tokio::test]
async fn owner_inputs_reject_noncanonical_revisions_and_ambiguous_queries_before_workflow() {
    let fixture = Harness::ready();
    let detail = format!("{OWNER}/{}", invitation());
    let resend_path = format!("{detail}/resend");
    for revision in [
        json!(0),
        json!(REVISION),
        json!("0"),
        json!("01"),
        json!("+1"),
        json!(" 1"),
        json!("-1"),
        json!("1.0"),
        json!("9223372036854775808"),
    ] {
        let mut body = resend();
        body["expected_revision"] = revision;
        assert_eq!(
            call(
                &fixture.router,
                "POST",
                &resend_path,
                Some("owner"),
                Some(body)
            )
            .await
            .status,
            StatusCode::BAD_REQUEST
        );
    }
    for (path, valid) in [(OWNER.to_string(), issue()), (resend_path, resend())] {
        for body in [
            json!([]),
            json!({}),
            {
                let mut v = valid.clone();
                v["password"] = json!(PASSWORD);
                v
            },
            {
                let mut v = valid.clone();
                v["operation_id"] = json!(uuid::Uuid::nil());
                v
            },
        ] {
            assert_eq!(
                call(&fixture.router, "POST", &path, Some("owner"), Some(body))
                    .await
                    .status,
                StatusCode::BAD_REQUEST
            );
        }
        assert_eq!(
            call(
                &fixture.router,
                "POST",
                &format!("{path}?extra=1"),
                Some("owner"),
                Some(valid)
            )
            .await
            .status,
            StatusCode::BAD_REQUEST
        );
    }
    for query in [
        "limit=0",
        "limit=101",
        "limit=1&limit=2",
        "after=bad",
        "unexpected=1",
        "after=00000000-0000-0000-0000-000000000000",
    ] {
        assert_eq!(
            call(
                &fixture.router,
                "GET",
                &format!("{OWNER}?{query}"),
                Some("owner"),
                None
            )
            .await
            .status,
            StatusCode::BAD_REQUEST
        );
    }
    for path in [
        format!("{detail}?extra=1"),
        format!("{OWNER}/not-uuid"),
        format!("{OWNER}/{}", uuid::Uuid::nil()),
    ] {
        assert_eq!(
            call(&fixture.router, "GET", &path, Some("owner"), None)
                .await
                .status,
            StatusCode::BAD_REQUEST
        );
    }
    assert!(fixture.workflow.calls().is_empty());
}
