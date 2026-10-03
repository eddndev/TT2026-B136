use super::support::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use serde_json::{json, Value};

async fn rejected(path: &str, bodies: Vec<String>) {
    let (router, workflow) = standalone();
    for body in bodies {
        let reply = post(&router, path, body.clone()).await;
        assert_eq!(
            reply.status,
            StatusCode::BAD_REQUEST,
            "accepted input: {body}"
        );
        public(&reply);
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

fn malformed_objects(valid: &str, fields: &[&str]) -> Vec<String> {
    let original: Value = serde_json::from_str(valid).unwrap();
    let mut bodies = vec![
        "null".into(),
        "[]".into(),
        "{}".into(),
        "true".into(),
        "\"text\"".into(),
        "{".into(),
        format!("{valid} {{}}"),
    ];
    bodies
        .push(Value::Array(fields.iter().map(|key| original[*key].clone()).collect()).to_string());
    for key in fields {
        let mut missing = original.clone();
        missing.as_object_mut().unwrap().remove(*key);
        bodies.push(missing.to_string());
        for replacement in [Value::Null, json!(19), json!(true), json!([]), json!({})] {
            let mut changed = original.clone();
            changed[*key] = replacement;
            bodies.push(changed.to_string());
        }
        bodies.push(format!(
            "{},\"{key}\":{}}}",
            &valid[..valid.len() - 1],
            original[*key]
        ));
    }
    for key in [
        "role",
        "certificate_base64",
        "trust",
        "success",
        "statement_base64",
        "now",
    ] {
        let mut extended = original.clone();
        extended[key] = json!("untrusted-client-authority");
        bodies.push(extended.to_string());
    }
    bodies
}

#[tokio::test]
async fn start_requires_one_strict_object_before_calling_the_workflow() {
    rejected(
        START,
        malformed_objects(&start_body(), &["owner_id", "binding_id"]),
    )
    .await;
}

#[tokio::test]
async fn proof_requires_one_strict_object_before_calling_the_workflow() {
    rejected(
        PROOF,
        malformed_objects(&proof_body(), &["challenge_token", "signature_base64"]),
    )
    .await;
}

#[tokio::test]
async fn start_rejects_nil_noncanonical_and_malformed_identity_in_either_field() {
    let mut bodies = Vec::new();
    for field in ["owner_id", "binding_id"] {
        let original: Value = serde_json::from_str(&start_body()).unwrap();
        let id = original[field].as_str().unwrap();
        for invalid in [
            String::new(),
            "00000000-0000-0000-0000-000000000000".into(),
            id.to_uppercase(),
            id.replace('-', ""),
            format!("urn:uuid:{id}"),
            format!("{{{id}}}"),
            format!(" {id}"),
            format!("{id}\n"),
            "not-an-id".into(),
        ] {
            let mut changed = original.clone();
            changed[field] = json!(invalid);
            bodies.push(changed.to_string());
        }
    }
    rejected(START, bodies).await;
}

#[tokio::test]
async fn proof_token_is_exact_canonical_unpadded_base64url_of_32_bytes() {
    let valid = token();
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let last = *valid.as_bytes().last().unwrap();
    let index = alphabet.iter().position(|byte| *byte == last).unwrap();
    let bad_bits = format!("{}{}", &valid[..42], alphabet[index + 1] as char);
    let bodies = [
        String::new(),
        "a".repeat(42),
        "a".repeat(44),
        format!("{valid}="),
        format!(" {valid}"),
        format!("{valid}\n"),
        STANDARD.encode([0xfb; 32]),
        URL_SAFE_NO_PAD.encode([0xfb; 31]),
        URL_SAFE_NO_PAD.encode([0xfb; 33]),
        bad_bits,
    ]
    .into_iter()
    .map(|invalid| {
        json!({"challenge_token": invalid,
            "signature_base64": STANDARD.encode(signature())})
        .to_string()
    })
    .collect();
    rejected(PROOF, bodies).await;
}

#[tokio::test]
async fn proof_signature_is_canonical_standard_base64_of_exactly_384_bytes() {
    let valid = STANDARD.encode(signature());
    let bodies = [
        String::new(),
        STANDARD.encode([0x41; 383]),
        STANDARD.encode([0x41; 385]),
        URL_SAFE_NO_PAD.encode(signature()),
        format!("{valid}="),
        format!(" {valid}"),
        format!("{valid}\n"),
        "!".repeat(512),
    ]
    .into_iter()
    .map(|invalid| {
        json!({"challenge_token": token(),
            "signature_base64": invalid})
        .to_string()
    })
    .collect();
    rejected(PROOF, bodies).await;
}

#[tokio::test]
async fn query_strings_and_non_post_methods_never_dispatch() {
    let (router, workflow) = standalone();
    for (path, body) in [(START, start_body()), (PROOF, proof_body())] {
        for suffix in ["?", "?owner_id=hidden", "?ignored=true"] {
            let reply = post(&router, &format!("{path}{suffix}"), body.clone()).await;
            assert_eq!(reply.status, StatusCode::BAD_REQUEST);
            public(&reply);
        }
        for method in ["GET", "PUT", "PATCH", "DELETE"] {
            let reply = send(
                &router,
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await;
            assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED);
            public(&reply);
        }
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn content_type_requires_one_json_header_and_accepts_shared_json_media_types() {
    for (path, body) in [(START, start_body()), (PROOF, proof_body())] {
        let (router, workflow) = standalone();
        for types in [
            vec![],
            vec!["text/plain"],
            vec!["application/json", "application/json"],
            vec!["application/json, text/plain"],
        ] {
            let mut request = Request::post(path);
            for value in types {
                request = request.header("content-type", value);
            }
            let reply = send(&router, request.body(Body::from(body.clone())).unwrap()).await;
            assert_eq!(reply.status, StatusCode::BAD_REQUEST);
            public(&reply);
        }
        assert!(workflow.calls.lock().unwrap().is_empty());
        for media in [
            "application/json",
            "Application/Json; charset=utf-8",
            "application/problem+json",
        ] {
            let reply = send(
                &router,
                Request::post(path)
                    .header("content-type", media)
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await;
            assert_eq!(reply.status, StatusCode::OK);
            public(&reply);
        }
        assert_eq!(workflow.calls.lock().unwrap().len(), 3);
    }
}
