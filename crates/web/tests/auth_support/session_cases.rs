use super::*;

const ROUTES: [(&str, &str); 2] = [
    ("GET", "/api/v1/auth/session"),
    ("POST", "/api/v1/auth/activity"),
];

async fn request(
    method: &str,
    path: &str,
    authorization: &[&str],
    body: Body,
) -> axum::response::Response {
    let mut request = Request::builder().method(method).uri(path);
    for value in authorization {
        request = request.header("authorization", *value);
    }
    router().oneshot(request.body(body).unwrap()).await.unwrap()
}

#[tokio::test]
async fn session_routes_require_one_live_bearer() {
    for (method, path) in ROUTES {
        for authorization in [
            vec![],
            vec!["Bearer invalid-token"],
            vec!["Bearer "],
            vec!["Basic owner-token"],
            vec!["Bearer owner-token", "Bearer client-token"],
        ] {
            let response = request(method, path, &authorization, Body::empty()).await;
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "{method} {path}"
            );
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert_eq!(json(response).await["error"]["code"], "invalid_session");
        }
    }
}

#[tokio::test]
async fn session_routes_accept_each_role_without_exposing_internal_identity() {
    for (method, path) in ROUTES {
        for role in ["owner", "litigator", "paralegal", "client"] {
            let authorization = format!("bearer {role}-token");
            let response = request(method, path, &[&authorization], Body::empty()).await;
            assert_eq!(response.status(), StatusCode::OK, "{method} {role}");
            assert_eq!(response.headers()["cache-control"], "no-store");
            let value = json(response).await;
            assert_eq!(value["user"]["role"], role);
            assert_eq!(value["policy"]["absolute_ttl_seconds"], 86_400);
            assert!(value["policy"].get("idle_ttl_seconds").unwrap().is_null());
            assert_eq!(value["server_now_unix_ms"], 100_000);
            assert_eq!(value["absolute_expires_at_unix_ms"], 86_500_000);
            assert!(value.get("idle_expires_at_unix_ms").unwrap().is_null());
            assert!(value.get("access_token").is_none());
            assert!(value.get("token_type").is_none());
            assert!(!value.to_string().contains("auth_generation"));
        }
    }
}

#[tokio::test]
async fn session_routes_reject_every_nonempty_body() {
    for (method, path) in ROUTES {
        for body in [
            " ",
            "{}",
            r#"{"server_now_unix_ms":0,"idle_ttl_seconds":86400}"#,
        ] {
            let response = request(method, path, &["Bearer owner-token"], Body::from(body)).await;
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "{method} {body}"
            );
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert_eq!(
                json(response).await["error"]["code"],
                "invalid_session_request"
            );
        }
    }
}

#[tokio::test]
async fn session_routes_reject_query_parameters() {
    for (method, path) in ROUTES {
        for query in ["?", "?idle_ttl_seconds=86400", "?user_id=another-user"] {
            let response = request(
                method,
                &format!("{path}{query}"),
                &["Bearer owner-token"],
                Body::empty(),
            )
            .await;
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "{method} {query}"
            );
            assert_eq!(
                json(response).await["error"]["code"],
                "invalid_session_request"
            );
        }
    }
}

#[tokio::test]
async fn session_routes_fail_closed_when_the_identity_port_is_unavailable() {
    for (method, path) in ROUTES {
        let response = request(method, path, &["Bearer unavailable-token"], Body::empty()).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let value = json(response).await;
        assert!(value.get("user").is_none());
        assert!(value.get("absolute_expires_at_unix_ms").is_none());
        assert!(!value.to_string().contains("private Redis failure"));
    }
}

#[tokio::test]
async fn only_the_activity_route_records_activity() {
    let identity = Arc::new(StubIdentity::default());
    let app = application_router(Arc::new(UnusedDocuments), identity.clone());
    for path in [
        "/api/v1/auth/me",
        "/api/v1/auth/session",
        "/api/v1/auth/session",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(path)
                    .header("authorization", "Bearer owner-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(identity.status_calls.load(Ordering::SeqCst), 2);
    assert_eq!(identity.activity_calls.load(Ordering::SeqCst), 0);
    let response = app
        .oneshot(
            Request::post("/api/v1/auth/activity")
                .header("authorization", "Bearer owner-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(identity.activity_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn both_second_factors_return_public_session_deadlines() {
    for (factor, code) in [("totp", "123456"), ("recovery", "RECOVERY-0")] {
        let response = router()
            .oneshot(
                Request::post(format!("/api/v1/auth/mfa/{factor}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "challenge_token": "challenge-token", "code": code
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value = json(response).await;
        assert_eq!(value["access_token"], "owner-token");
        assert_eq!(value["token_type"], "Bearer");
        assert_eq!(value["expires_in_seconds"], 86_400);
        assert_eq!(value["policy"]["absolute_ttl_seconds"], 86_400);
        assert!(value["policy"].get("idle_ttl_seconds").unwrap().is_null());
        assert_eq!(value["server_now_unix_ms"], 100_000);
        assert_eq!(value["absolute_expires_at_unix_ms"], 86_500_000);
        assert!(value.get("idle_expires_at_unix_ms").unwrap().is_null());
        assert!(!value.to_string().contains("auth_generation"));
    }
}
