#[path = "member_invitations_support/boundary.rs"]
mod boundary;
#[path = "member_invitations_support/capacity.rs"]
mod capacity;
mod member_invitations_support;

use application::identity::invitations::*;
use axum::http::StatusCode;
use domain::identity::Role;
use member_invitations_support::*;
use serde_json::json;

#[tokio::test]
async fn owner_routes_delegate_exact_intent_and_return_only_metadata() {
    let fixture = Harness::ready();
    let id = InvitationId::from_uuid(invitation());
    let detail = format!("{OWNER}/{id}");
    let reply = call(&fixture.router, "POST", OWNER, Some("owner"), Some(issue())).await;
    assert_eq!(reply.status, StatusCode::CREATED);
    reply.metadata();
    assert_eq!(reply.json(), summary());
    let reply = call(&fixture.router, "GET", &detail, Some("owner"), None).await;
    assert_eq!(reply.status, StatusCode::OK);
    reply.metadata();
    assert_eq!(reply.json(), summary());
    let reply = call(
        &fixture.router,
        "POST",
        &format!("{detail}/resend"),
        Some("owner"),
        Some(resend()),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    reply.metadata();
    assert_eq!(reply.json(), summary());
    let after = InvitationId::from_uuid(Uuid::from_u128(21));
    let reply = call(
        &fixture.router,
        "GET",
        &format!("{OWNER}?limit=1&after={after}"),
        Some("owner"),
        None,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    reply.metadata();
    assert_eq!(
        reply.json(),
        json!({"items":[summary()],"next":invitation()})
    );
    assert_eq!(
        fixture.workflow.calls(),
        vec![
            Call::Issue(
                "owner".into(),
                InvitationIssueRequest {
                    operation_id: operation(),
                    email: "staff@example.test".into(),
                    role: Role::Litigator,
                }
            ),
            Call::Get("owner".into(), id),
            Call::Resend(
                "owner".into(),
                id,
                InvitationResendRequest {
                    operation_id: operation(),
                    expected_revision: REVISION,
                }
            ),
            Call::List(
                "owner".into(),
                InvitationQuery {
                    after: Some(after),
                    limit: 1
                }
            ),
        ]
    );
}

#[tokio::test]
async fn owner_routes_require_bearer_and_delegate_role_denial() {
    let fixture = Harness::ready();
    let detail = format!("{OWNER}/{}", invitation());
    for (method, path, body) in [
        ("GET", OWNER.to_string(), None),
        ("POST", OWNER.into(), Some(issue())),
        ("GET", detail.clone(), None),
        ("POST", format!("{detail}/resend"), Some(resend())),
    ] {
        call(&fixture.router, method, &path, None, body.clone())
            .await
            .error(StatusCode::UNAUTHORIZED, "invalid_session");
        assert!(fixture.workflow.calls().is_empty());
    }
    for bearer in ["litigator", "paralegal", "client"] {
        call(&fixture.router, "POST", OWNER, Some(bearer), Some(issue()))
            .await
            .error(StatusCode::FORBIDDEN, "permission_denied");
    }
    assert_eq!(fixture.workflow.calls().len(), 3);
}

#[tokio::test]
async fn public_enrollment_and_receipt_contracts_never_issue_a_session() {
    let fixture = Harness::ready();
    let reply = call(
        &fixture.router,
        "POST",
        &format!("{PUBLIC}/prepare"),
        None,
        Some(recipient()),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    reply.private();
    assert_eq!(
        reply.json(),
        json!({
            "id":invitation(),"revision":REVISION.to_string(),"operation_id":operation(),
            "email":"staff@example.test","role":"litigator","expires_at":"1970-01-02T00:00:00Z",
            "totp_secret_base32":SECRET,"otpauth_uri":format!("otpauth://totp/staff?secret={SECRET}"),
            "recovery_codes":["PRIVATE-RECOVERY-CODE"]
        })
    );
    let mut finish = recipient();
    finish["code"] = json!(CODE);
    let reply = call(
        &fixture.router,
        "POST",
        &format!("{PUBLIC}/finish"),
        None,
        Some(finish),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    reply.metadata();
    assert_eq!(reply.json(), receipt());
    let reply = call(
        &fixture.router,
        "POST",
        &format!("{PUBLIC}/receipt"),
        None,
        Some(recipient()),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    reply.metadata();
    assert_eq!(reply.json(), json!({"receipt":receipt()}));
    *fixture.workflow.mode.lock().unwrap() = Mode::MissingReceipt;
    let reply = call(
        &fixture.router,
        "POST",
        &format!("{PUBLIC}/receipt"),
        None,
        Some(recipient()),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    reply.metadata();
    assert_eq!(reply.json(), json!({"receipt":null}));
    assert_eq!(
        fixture.workflow.calls(),
        vec![
            Call::Recipient("prepare"),
            Call::Recipient("finish"),
            Call::Recipient("receipt"),
            Call::Recipient("receipt")
        ]
    );
}

#[tokio::test]
async fn public_rejection_and_uncertainty_are_sanitized_without_retrying() {
    let fixture = Harness::ready();
    for (mode, status, code) in [
        (
            Mode::Rejected,
            StatusCode::BAD_REQUEST,
            "invitation_rejected",
        ),
        (
            Mode::MfaRejected,
            StatusCode::BAD_REQUEST,
            "invitation_rejected",
        ),
        (
            Mode::Conflict,
            StatusCode::BAD_REQUEST,
            "invitation_rejected",
        ),
        (
            Mode::InvalidInput,
            StatusCode::BAD_REQUEST,
            "invitation_rejected",
        ),
        (
            Mode::Uncertain,
            StatusCode::SERVICE_UNAVAILABLE,
            "invitation_uncertain",
        ),
    ] {
        *fixture.workflow.mode.lock().unwrap() = mode;
        for action in ["prepare", "finish", "receipt"] {
            let mut body = recipient();
            if action == "finish" {
                body["code"] = json!(CODE);
            }
            let before = fixture.workflow.calls().len();
            call(
                &fixture.router,
                "POST",
                &format!("{PUBLIC}/{action}"),
                None,
                Some(body),
            )
            .await
            .error(status, code);
            assert_eq!(
                fixture.workflow.calls().len(),
                before + 1,
                "HTTP retried a recipient operation"
            );
        }
    }
}

#[tokio::test]
async fn absent_capability_has_no_effect_and_returns_no_secrets() {
    let fixture = Harness::new(false, 2);
    for action in ["prepare", "finish", "receipt"] {
        let mut body = recipient();
        if action == "finish" {
            body["code"] = json!(CODE);
        }
        call(
            &fixture.router,
            "POST",
            &format!("{PUBLIC}/{action}"),
            None,
            Some(body),
        )
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "invitation_unavailable");
    }
    call(&fixture.router, "POST", OWNER, Some("owner"), Some(issue()))
        .await
        .error(StatusCode::SERVICE_UNAVAILABLE, "invitation_unavailable");
    assert!(fixture.workflow.calls().is_empty());
}
