use super::*;

const OWNER_USER: &str = r#"{"id":"00000000-0000-0000-0000-000000000009","email":"owner@example.com","name":"Owner Example","role":"owner"}"#;

async fn send(method: &str, path: &str, bearer: Option<&str>, body: Body) -> (StatusCode, String) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = bearer {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = router().oneshot(request.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

fn bootstrap_body(name: Option<&str>) -> Body {
    let mut value = serde_json::json!({
        "email": "owner@example.com",
        "password": "correct horse battery",
    });
    if let Some(name) = name {
        value["name"] = name.into();
    }
    Body::from(value.to_string())
}

fn create_body(name: Option<&str>) -> Body {
    let mut value = serde_json::json!({
        "email": "staff@example.com",
        "password": "correct horse battery",
        "role": "litigator",
    });
    if let Some(name) = name {
        value["name"] = name.into();
    }
    Body::from(value.to_string())
}

fn value(body: &str) -> Value {
    serde_json::from_str(body).unwrap()
}

#[tokio::test]
async fn enrollment_routes_accept_a_name_and_return_it_with_the_account() {
    let (status, body) = send(
        "POST",
        "/api/v1/auth/bootstrap",
        None,
        bootstrap_body(Some("  Ana Garc\u{ed}a ")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(value(&body)["user"]["name"], "Ana Garc\u{ed}a");
    let (status, body) = send(
        "POST",
        "/api/v1/users",
        Some("owner-token"),
        create_body(Some("Luis Pe\u{f1}a")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let user = &value(&body)["user"];
    assert_eq!(user["name"], "Luis Pe\u{f1}a");
    assert_eq!(user["role"], "litigator");
}

#[tokio::test]
async fn enrollment_routes_require_a_name_field() {
    for (path, bearer, body) in [
        ("/api/v1/auth/bootstrap", None, bootstrap_body(None)),
        ("/api/v1/users", Some("owner-token"), create_body(None)),
    ] {
        let (status, _) = send("POST", path, bearer, body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{path}");
    }
}

#[tokio::test]
async fn invalid_names_have_a_stable_unprocessable_error() {
    let long = "a".repeat(121);
    for name in ["", "   ", "Ana\u{202e}", "\tAna", long.as_str()] {
        for (path, bearer, body) in [
            ("/api/v1/auth/bootstrap", None, bootstrap_body(Some(name))),
            (
                "/api/v1/users",
                Some("owner-token"),
                create_body(Some(name)),
            ),
        ] {
            let (status, body) = send("POST", path, bearer, body).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{path} {name:?}");
            let error = &value(&body)["error"];
            assert_eq!(error["code"], "invalid_display_name", "{path} {name:?}");
            assert!(error["message"]
                .as_str()
                .unwrap()
                .starts_with("invalid display name: "));
        }
    }
}

#[tokio::test]
async fn every_session_user_payload_carries_the_name_in_a_fixed_order() {
    let (status, body) = send("GET", "/api/v1/auth/me", Some("owner-token"), Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, OWNER_USER);
    for (method, path) in [
        ("GET", "/api/v1/auth/session"),
        ("POST", "/api/v1/auth/activity"),
    ] {
        let (status, body) = send(method, path, Some("owner-token"), Body::empty()).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(body.contains(&format!(r#""user":{OWNER_USER}"#)), "{path}");
    }
    for (factor, code) in [("totp", "123456"), ("recovery", "RECOVERY-0")] {
        let body = serde_json::json!({"challenge_token": "challenge-token", "code": code});
        let (status, body) = send(
            "POST",
            &format!("/api/v1/auth/mfa/{factor}"),
            None,
            Body::from(body.to_string()),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{factor}");
        assert!(
            body.contains(&format!(r#""user":{OWNER_USER}"#)),
            "{factor}"
        );
    }
}

#[tokio::test]
async fn login_still_rejects_a_name_field() {
    let body = serde_json::json!({
        "email": "owner@example.com",
        "name": "Owner Example",
        "password": "correct horse battery",
    });
    let (status, _) = send(
        "POST",
        "/api/v1/auth/login",
        None,
        Body::from(body.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
