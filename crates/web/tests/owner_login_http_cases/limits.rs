use super::support::*;
use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode},
};

fn padded(body: &str, limit: usize) -> String {
    assert!(body.len() <= limit);
    format!("{body}{}", " ".repeat(limit - body.len()))
}

#[tokio::test]
async fn complete_entity_caps_accept_exact_bytes_and_reject_one_more_before_dispatch() {
    for (path, body, limit) in [(START, start_body(), 1024), (PROOF, proof_body(), 2048)] {
        let (router, workflow) = standalone();
        let exact = padded(&body, limit);
        let reply = post(&router, path, exact.clone()).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(workflow.calls.lock().unwrap().len(), 1);
        workflow.calls.lock().unwrap().clear();
        let reply = post(&router, path, format!("{exact} ")).await;
        assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
        public(&reply);
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn caps_measure_chunked_bytes_and_do_not_trust_content_length() {
    for (path, body, limit) in [(START, start_body(), 1024), (PROOF, proof_body(), 2048)] {
        let (router, workflow) = standalone();
        for advertised in [None, Some("1"), Some("0")] {
            let oversized = padded(&body, limit + 1);
            let chunks: Vec<Result<Bytes, std::io::Error>> = oversized
                .as_bytes()
                .chunks(137)
                .map(|chunk| Ok(Bytes::copy_from_slice(chunk)))
                .collect();
            let mut request = Request::post(path).header("content-type", "application/json");
            if let Some(length) = advertised {
                request = request.header("content-length", length);
            }
            let reply = send(
                &router,
                request
                    .body(Body::from_stream(futures_util::stream::iter(chunks)))
                    .unwrap(),
            )
            .await;
            assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
            public(&reply);
        }
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn failed_body_read_never_dispatches_or_echoes_transport_details() {
    for path in [START, PROOF] {
        let (router, workflow) = standalone();
        let chunks = futures_util::stream::iter([
            Ok(Bytes::from_static(b"{")),
            Err(std::io::Error::other("private-owner-body-error")),
        ]);
        let reply = send(
            &router,
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from_stream(chunks))
                .unwrap(),
        )
        .await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST);
        public(&reply);
        assert!(!String::from_utf8(reply.body)
            .unwrap()
            .contains("private-owner-body-error"));
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
