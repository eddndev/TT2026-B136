use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Method, Request, StatusCode},
    Router,
};
use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::runtime::Runtime;
use tower::ServiceExt;
use uuid::Uuid;

pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn json(&self, expected: StatusCode) -> Value {
        assert_eq!(self.status, expected);
        assert_eq!(self.headers["cache-control"], "no-store");
        serde_json::from_slice(&self.body).unwrap()
    }
}

pub fn send(
    runtime: &Runtime,
    router: &Router,
    method: Method,
    path: &str,
    token: &str,
    body: Option<Value>,
) -> Reply {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap();
    runtime.block_on(async {
        let response =
            tokio::time::timeout(Duration::from_secs(90), router.clone().oneshot(request))
                .await
                .expect("report HTTP acceptance watchdog")
                .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), 16 * 1024 * 1024)
            .await
            .unwrap()
            .to_vec();
        Reply {
            status,
            headers,
            body,
        }
    })
}

pub fn id(value: &Value) -> Uuid {
    value["id"].as_str().unwrap().parse().unwrap()
}

pub fn stable(value: &Value) -> Value {
    let mut value = value.clone();
    value.as_object_mut().unwrap().remove("ready_email");
    value
}

pub fn download(reply: Reply, report: &Value, format: &str) -> Vec<u8> {
    assert_eq!(reply.status, StatusCode::OK);
    let expected = report["ready"]["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|artifact| artifact["format"] == format)
        .unwrap();
    let digest = RingSha256Hasher.hash_bytes(&reply.body).to_hex();
    assert_eq!(json!(reply.body.len()), expected["bytes"]);
    assert_eq!(digest, expected["digest"].as_str().unwrap());
    assert_eq!(reply.headers["x-report-digest"], digest);
    assert_eq!(reply.headers["x-report-id"], report["id"].as_str().unwrap());
    assert_eq!(
        reply.headers["x-report-snapshot-digest"],
        report["ready"]["snapshot_digest"].as_str().unwrap()
    );
    assert_eq!(reply.headers["cache-control"], "no-store");
    assert_eq!(reply.headers["x-content-type-options"], "nosniff");
    assert_eq!(
        reply.headers["content-length"],
        reply.body.len().to_string()
    );
    assert_eq!(
        reply.headers["content-disposition"],
        format!("attachment; filename=\"report-{}.{}\"", id(report), format)
    );
    if format == "pdf" {
        assert_eq!(reply.headers["content-type"], "application/pdf");
        assert!(reply.body.starts_with(b"%PDF-"));
    } else {
        assert_eq!(reply.headers["content-type"], "text/csv; charset=utf-8");
        assert!(String::from_utf8_lossy(&reply.body).contains(report["id"].as_str().unwrap()));
    }
    reply.body
}
