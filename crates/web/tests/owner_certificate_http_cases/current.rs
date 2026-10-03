use application::ApplicationError;
use axum::http::StatusCode;
use domain::identity::Role;

use crate::{owner_certificate_http_support::*, support::*};

const CURRENT: &str = "/api/v1/auth/certificate-bindings/current";

#[tokio::test]
async fn current_route_returns_exact_unwithdrawn_receipt_or_null_and_keeps_uuid_history() {
    for found in [None, Some(receipt())] {
        let mut harness = Harness::new();
        let observed = harness.state.clone();
        {
            let mut state = observed.lock().unwrap();
            state.context.trust = None;
            state.now = 4000;
            state.found = Some(retired());
        }
        let expected = found.clone();
        let calls = observed.clone();
        harness
            .store
            .expect_find_current()
            .times(1)
            .returning(move |actor| {
                assert_eq!(actor, principal().id);
                calls.lock().unwrap().calls.push("find_current");
                Ok(found.clone())
            });
        let app = router(harness);
        let response = raw(&app, "GET", CURRENT, vec![], &[], &["Bearer session"]).await;
        response.public();
        assert_eq!(response.status, StatusCode::OK);
        assert!(response.headers["content-type"]
            .to_str()
            .unwrap()
            .starts_with("application/json"));
        match expected {
            Some(value) => assert_evidence(&response.json(), &value),
            None => assert!(response.json().is_null()),
        }
        assert_eq!(
            observed.lock().unwrap().calls,
            ["authenticate", "find_current", "authenticate"]
        );
        let historical = request(&app, "GET", "", None).await;
        historical.public();
        assert_eq!(historical.status, StatusCode::OK);
        assert_evidence(&historical.json(), &retired());
    }
}

#[tokio::test]
async fn current_route_rejects_entities_queries_ambiguous_bearers_and_non_owner_authority() {
    let harness = Harness::new();
    let observed = harness.state.clone();
    let app = router(harness);
    for body in [b"{}".to_vec(), b"\n".to_vec()] {
        raw(
            &app,
            "GET",
            CURRENT,
            body,
            &["application/json"],
            &["Bearer session"],
        )
        .await
        .error(StatusCode::BAD_REQUEST, "owner_certificate_invalid_input");
    }
    for query in ["?", "?owner_id=untrusted", "?current=true"] {
        raw(
            &app,
            "GET",
            &format!("{CURRENT}{query}"),
            vec![],
            &[],
            &["Bearer session"],
        )
        .await
        .error(StatusCode::BAD_REQUEST, "owner_certificate_invalid_input");
    }
    for auth in [
        vec![],
        vec!["Basic session"],
        vec!["Bearer session", "Bearer session"],
    ] {
        raw(&app, "GET", CURRENT, vec![], &[], &auth)
            .await
            .error(StatusCode::UNAUTHORIZED, "invalid_session");
    }
    assert!(observed.lock().unwrap().calls.is_empty());
    for role in [
        None,
        Some(Role::Litigator),
        Some(Role::Paralegal),
        Some(Role::Client),
    ] {
        let harness = Harness::new();
        let observed = harness.state.clone();
        observed.lock().unwrap().principal = role.map(|role| application::identity::Principal {
            role,
            ..principal()
        });
        let response = raw(
            &router(harness),
            "GET",
            CURRENT,
            vec![],
            &[],
            &["Bearer session"],
        )
        .await;
        if role.is_none() {
            response.error(StatusCode::UNAUTHORIZED, "invalid_session");
        } else {
            response.error(StatusCode::FORBIDDEN, "permission_denied");
        }
        assert_eq!(observed.lock().unwrap().calls, ["authenticate"]);
    }
}

#[tokio::test]
async fn current_route_does_not_release_evidence_or_absence_after_session_loss_or_port_error() {
    for variant in 0..3 {
        let mut harness = Harness::new();
        let observed = harness.state.clone();
        let calls = observed.clone();
        harness
            .store
            .expect_find_current()
            .times(1)
            .returning(move |actor| {
                assert_eq!(actor, principal().id);
                let mut state = calls.lock().unwrap();
                state.calls.push("find_current");
                if variant == 2 {
                    return Err(ApplicationError::Port("private-port-detail".into()));
                }
                state.principal = None;
                Ok((variant == 1).then(receipt))
            });
        let response = raw(
            &router(harness),
            "GET",
            CURRENT,
            vec![],
            &[],
            &["Bearer session"],
        )
        .await;
        if variant == 2 {
            response.error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error");
        } else {
            response.error(StatusCode::UNAUTHORIZED, "invalid_session");
        }
        assert!(!String::from_utf8_lossy(&response.bytes).contains("Synthetic Partner"));
        let mut expected = vec!["authenticate", "find_current"];
        if variant != 2 {
            expected.push("authenticate");
        }
        assert_eq!(observed.lock().unwrap().calls, expected);
    }
}
