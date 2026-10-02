use crate::password_reset_http_support::*;
use axum::{body::Body, http::StatusCode};

#[tokio::test]
async fn accepted_busy_and_malformed_email_have_identical_public_responses_without_service_work() {
    let mut baseline = None;
    for admission in [RequestAdmission::Accepted, RequestAdmission::Busy] {
        for value in [
            "private@example.test",
            "",
            "not an email",
            " OWNER@EXAMPLE.TEST ",
        ] {
            let fixture = Harness::new(admission, true, 2, 1);
            let reply = post(&fixture.router, REQUEST, email(value)).await;
            reply.public();
            assert_eq!(reply.status, StatusCode::ACCEPTED);
            assert_eq!(reply.json(), serde_json::json!({"status":"accepted"}));
            if let Some(expected) = &baseline {
                assert!(
                    &reply == expected,
                    "admission disposition changed the public response"
                );
            } else {
                baseline = Some(reply);
            }
            assert_eq!(*fixture.requests.calls.lock().unwrap(), 1);
            assert!(
                fixture.ports.calls().is_empty(),
                "request handler awaited completion ports"
            );
            let retained = fixture.requests.retained.lock().unwrap();
            match admission {
                RequestAdmission::Accepted => {
                    assert_eq!(retained.len(), 1);
                    assert!(
                        retained[0].as_str() == value,
                        "HTTP changed the submitted email"
                    );
                }
                RequestAdmission::Busy => {
                    assert!(retained.is_empty(), "busy admission queued work")
                }
                RequestAdmission::Unavailable => unreachable!(),
            }
        }
    }
}

#[tokio::test]
async fn disabled_components_and_unavailable_admission_return_only_global_unavailability() {
    let mut baseline = None;
    for enabled in [false, true] {
        for value in ["private@example.test", "not an email"] {
            let fixture = Harness::new(RequestAdmission::Unavailable, enabled, 2, 1);
            let reply = post(&fixture.router, REQUEST, email(value)).await;
            reply.error(
                StatusCode::SERVICE_UNAVAILABLE,
                "password_reset_unavailable",
            );
            if let Some(expected) = &baseline {
                assert!(&reply == expected);
            } else {
                baseline = Some(reply);
            }
            assert!(fixture.ports.calls().is_empty());
            assert!(fixture.requests.retained.lock().unwrap().is_empty());
            assert_eq!(
                *fixture.requests.calls.lock().unwrap(),
                usize::from(enabled)
            );
        }
    }
    let fixture = Harness::new(RequestAdmission::Accepted, false, 2, 1);
    post(&fixture.router, COMPLETE, completion(&token(), PASSWORD))
        .await
        .error(
            StatusCode::SERVICE_UNAVAILABLE,
            "password_reset_unavailable",
        );
    fixture.untouched();
}

#[tokio::test]
async fn strict_json_media_and_query_boundaries_reject_before_any_reset_work() {
    for path in [REQUEST, COMPLETE] {
        let valid = if path == REQUEST {
            email("private@example.test")
        } else {
            completion(&token(), PASSWORD)
        };
        let mut unknown: serde_json::Value = serde_json::from_str(&valid).unwrap();
        unknown["extra"] = serde_json::json!("private-reset-detail");
        let field = if path == REQUEST { "email" } else { "token" };
        let duplicate = valid.replace(
            &format!("\"{field}\":"),
            &format!("\"{field}\":\"private-reset-detail\",\"{field}\":"),
        );
        let positional = if path == REQUEST {
            serde_json::json!(["private@example.test"])
        } else {
            serde_json::json!([token(), PASSWORD])
        };
        for body in [
            "{".into(),
            "null".into(),
            "[]".into(),
            "{}".into(),
            positional.to_string(),
            format!("{valid}{{}}"),
            unknown.to_string(),
            duplicate,
            format!("{{\"{field}\":23}}"),
        ] {
            let fixture = Harness::ready();
            let reply = post(&fixture.router, path, body).await;
            reply.public();
            assert_eq!(reply.status, StatusCode::BAD_REQUEST);
            fixture.untouched();
        }
        for headers in [
            vec![],
            vec!["text/plain"],
            vec!["application/json", "application/json"],
        ] {
            let fixture = Harness::ready();
            let reply = raw(&fixture.router, path, Body::from(valid.clone()), &headers).await;
            reply.public();
            assert_eq!(reply.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
            fixture.untouched();
        }
        for suffix in ["?", "?email=private-reset-detail"] {
            let fixture = Harness::ready();
            let reply = post(&fixture.router, &format!("{path}{suffix}"), valid.clone()).await;
            reply.public();
            assert_eq!(reply.status, StatusCode::BAD_REQUEST);
            fixture.untouched();
        }
    }
}

#[tokio::test]
async fn sixteen_kibibytes_include_whitespace_and_one_extra_byte_never_reaches_ports() {
    for path in [REQUEST, COMPLETE] {
        let valid = if path == REQUEST {
            email("private@example.test")
        } else {
            completion(&token(), PASSWORD)
        };
        let exact = format!("{}{}", valid, " ".repeat(16 * 1024 - valid.len()));
        let fixture = Harness::ready();
        let reply = post(&fixture.router, path, exact.clone()).await;
        reply.public();
        assert_eq!(
            reply.status,
            if path == REQUEST {
                StatusCode::ACCEPTED
            } else {
                StatusCode::NO_CONTENT
            }
        );
        let fixture = Harness::ready();
        let reply = post(&fixture.router, path, format!("{exact} ")).await;
        reply.public();
        assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
        fixture.untouched();
    }
}
