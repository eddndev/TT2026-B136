mod procedural_fact_http_support;
use procedural_fact_http_support::*;
use serde_json::{json, Value};
use std::sync::Arc;

async fn raw(family: &str, text: String, types: &[&str]) -> (u16, Value, usize) {
    let workflow = Arc::new(Workflow::default());
    let (status, body) = request(
        workflow.clone(),
        "POST",
        &format!("{}/prepare", base_url(family)),
        Some("capture"),
        Some(text),
        types,
    )
    .await;
    let calls = workflow.calls.lock().unwrap().len();
    (status, body, calls)
}

#[tokio::test]
async fn json_requires_one_content_type_one_named_object_and_no_trailing_value() {
    for family in ["resolution", "notification"] {
        let c = command_json(family, "record");
        for headers in [
            vec![],
            vec!["text/plain"],
            vec!["application/json", "application/json"],
        ] {
            let (status, _, calls) = raw(family, c.to_string(), &headers).await;
            assert_eq!(status, 400);
            assert_eq!(calls, 0);
        }
        for body in [
            "null".into(),
            "[]".into(),
            "{}".into(),
            format!("{c}{{}}"),
            format!("{c}null"),
            format!("{c} trailing"),
        ] {
            let (status, response, calls) = raw(family, body, &["application/json"]).await;
            assert_eq!(status, 400, "{response}");
            assert_eq!(calls, 0);
        }
    }
}

#[tokio::test]
async fn unknown_duplicate_and_array_objects_are_rejected_at_every_nested_boundary() {
    for vector in vectors() {
        let family = vector["family"].as_str().unwrap();
        let mut c = command_json(family, "record");
        c["change"]["values"] = vector["normalized"].clone();
        if family == "notification" {
            c["change"]["values"]["resolution"]["id"] = json!(PARENT);
        }
        let mut paths = Vec::new();
        object_paths(&c, "", &mut paths);
        for path in paths {
            let mut unknown = c.clone();
            unknown
                .pointer_mut(&path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unexpected".into(), json!(true));
            let mut array = c.clone();
            let fields = array
                .pointer(&path)
                .unwrap()
                .as_object()
                .unwrap()
                .values()
                .cloned()
                .collect::<Vec<_>>();
            *array.pointer_mut(&path).unwrap() = json!(fields);
            for body in [unknown.to_string(), array.to_string(), duplicate(&c, &path)] {
                let (status, response, calls) = raw(family, body, &["application/json"]).await;
                assert_eq!(status, 400, "{} {path}: {response}", vector["name"]);
                assert_eq!(calls, 0, "{} {path}", vector["name"]);
            }
        }
    }
}

#[tokio::test]
async fn submission_wrapper_and_withdrawal_cannot_carry_hidden_replacement_values() {
    for family in ["resolution", "notification"] {
        let original = submission(command_json(family, "withdraw"));
        let mut extra = original.clone();
        extra["unexpected"] = json!(true);
        let mut replacement = original.clone();
        replacement["command"]["change"]["values"] = values_json(family);
        let missing = json!({"command":command_json(family,"record")});
        for body in [
            extra.to_string(),
            replacement.to_string(),
            missing.to_string(),
            duplicate(&original, ""),
        ] {
            let workflow = Arc::new(Workflow::default());
            let (status, response) = request(
                workflow.clone(),
                "POST",
                &format!("{}/{ID}/withdrawal", base_url(family)),
                Some("valid"),
                Some(body),
                &["application/json"],
            )
            .await;
            assert_eq!(status, 400, "{response}");
            assert!(workflow.calls.lock().unwrap().is_empty());
        }
    }
}

#[tokio::test]
async fn full_512_kib_body_is_accepted_and_one_more_byte_is_rejected() {
    for family in ["resolution", "notification"] {
        let c = command_json(family, "record").to_string();
        let padded = format!("{c}{}", " ".repeat(512 * 1024 - c.len()));
        let (_, body, calls) = raw(family, padded.clone(), &["application/json"]).await;
        assert_eq!(calls, 1, "{body}");
        let (status, body, calls) = raw(family, format!("{padded} "), &["application/json"]).await;
        assert_eq!(status, 413, "{body}");
        assert_eq!(body["error"]["code"], "procedural_fact_body_too_large");
        assert_eq!(calls, 0);
    }
}

#[tokio::test]
async fn bearer_validation_precedes_body_decoding_and_path_validation() {
    for family in ["resolution", "notification"] {
        let path = format!("{}/prepare", base_url(family).replace(CASE, "bad-case"));
        for authorization in [
            vec![],
            vec![("authorization", "Basic abc".into())],
            vec![("authorization", "Bearer ".into())],
            vec![
                ("authorization", "Bearer valid".into()),
                ("authorization", "Bearer other".into()),
            ],
        ] {
            let workflow = Arc::new(Workflow::default());
            let (status, body) = request_headers(
                workflow.clone(),
                "POST",
                &path,
                Some("{not-json".into()),
                &authorization,
            )
            .await;
            assert_eq!(status, 401, "{body}");
            assert!(workflow.calls.lock().unwrap().is_empty());
        }
    }
}

fn object_paths(value: &Value, path: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(fields) => {
            out.push(path.to_owned());
            for (key, value) in fields {
                object_paths(value, &format!("{path}/{key}"), out);
            }
        }
        Value::Array(values) => {
            for (i, value) in values.iter().enumerate() {
                object_paths(value, &format!("{path}/{i}"), out);
            }
        }
        _ => {}
    }
}
fn duplicate(value: &Value, path: &str) -> String {
    fn render(value: &Value, current: &str, target: &str) -> String {
        match value {
            Value::Object(fields) => {
                let mut entries = fields
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}:{}",
                            json!(k),
                            render(v, &format!("{current}/{k}"), target)
                        )
                    })
                    .collect::<Vec<_>>();
                if current == target {
                    entries.push(entries.first().unwrap().clone());
                }
                format!("{{{}}}", entries.join(","))
            }
            Value::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .enumerate()
                    .map(|(i, v)| render(v, &format!("{current}/{i}"), target))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            _ => value.to_string(),
        }
    }
    render(value, "", path)
}

#[tokio::test]
async fn streamed_body_without_content_length_stops_at_the_same_byte_limit() {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    for family in ["resolution", "notification"] {
        let workflow = Arc::new(Workflow::default());
        let chunks = futures_util::stream::iter(vec![
            Ok::<_, std::io::Error>(" ".repeat(256 * 1024)),
            Ok(" ".repeat(256 * 1024)),
            Ok(command_json(family, "record").to_string()),
        ]);
        let req = Request::builder()
            .method("POST")
            .uri(format!("{}/prepare", base_url(family)))
            .header("authorization", "Bearer valid")
            .header("content-type", "application/json")
            .body(Body::from_stream(chunks))
            .unwrap();
        assert!(!req.headers().contains_key("content-length"));
        let response = web::procedural_fact_router(workflow.clone())
            .oneshot(req)
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 413);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(body["error"]["code"], "procedural_fact_body_too_large");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
