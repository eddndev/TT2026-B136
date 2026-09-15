mod document_support;

use std::sync::{atomic::Ordering, Arc};

use axum::body::{to_bytes, Body, Bytes};
use axum::http::{Request, StatusCode};
use document_support::{StubIdentity, StubWorkflow, CASE_UUID};
use serde_json::{json, Value};
use tower::ServiceExt;

const BOUNDARY: &str = "document-metadata-fixture";
const FILE_LIMIT: usize = 16 * 1024 * 1024;

fn metadata() -> Vec<u8> {
    json!({"document_type":" Escrito ","classification":" Penal ","tags":["a,b","acci\u{00f3}n"]})
        .to_string()
        .into_bytes()
}

fn multipart(parts: &[(&str, &[u8])]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (name, body) in parts {
        bytes.extend_from_slice(format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"ignored-name.bin\"\r\n\r\n").as_bytes());
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    bytes
}

async fn upload(
    workflow: &Arc<StubWorkflow>,
    token: Option<&str>,
    content_type: &str,
    body: Body,
) -> axum::response::Response {
    let mut request = Request::post(format!("/api/v1/cases/{CASE_UUID}/documents/with-metadata"))
        .header("content-type", content_type)
        .header("x-document-name", "acta.txt");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    web::application_router(workflow.clone(), Arc::new(StubIdentity))
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}

async fn send(workflow: &Arc<StubWorkflow>, bytes: Vec<u8>) -> axum::response::Response {
    upload(
        workflow,
        Some("owner-token"),
        &format!("multipart/form-data; boundary={BOUNDARY}"),
        Body::from(bytes),
    )
    .await
}

async fn json_body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}

#[tokio::test]
async fn classified_upload_accepts_either_part_order_and_returns_one_confirmed_overview() {
    let workflow = Arc::new(StubWorkflow::default());
    let metadata = metadata();
    for parts in [
        vec![
            ("file", b"case document".as_slice()),
            ("metadata", metadata.as_slice()),
        ],
        vec![
            ("metadata", metadata.as_slice()),
            ("file", b"case document".as_slice()),
        ],
    ] {
        let response = send(&workflow, multipart(&parts)).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = json_body(response).await;
        assert_eq!(body["name"], "acta.txt");
        assert_eq!(body["version"], 1);
        assert_eq!(body["current_metadata"]["metadata_revision"], 1);
        assert_eq!(body["current_metadata"]["document_type"], "Escrito");
        assert_eq!(
            body["current_metadata"]["tags"],
            json!(["a,b", "acci\u{00f3}n"])
        );
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn malformed_missing_repeated_and_unknown_parts_cannot_start_the_upload_workflow() {
    let workflow = Arc::new(StubWorkflow::default());
    let metadata = metadata();
    for parts in [
        vec![],
        vec![("file", b"case document".as_slice())],
        vec![("metadata", metadata.as_slice())],
        vec![
            ("file", b"case document".as_slice()),
            ("metadata", metadata.as_slice()),
            ("file", b"extra".as_slice()),
        ],
        vec![
            ("file", b"case document".as_slice()),
            ("metadata", metadata.as_slice()),
            ("metadata", metadata.as_slice()),
        ],
        vec![
            ("file", b"case document".as_slice()),
            ("metadata", metadata.as_slice()),
            ("actor", b"forged".as_slice()),
        ],
        vec![
            ("file", b"case document".as_slice()),
            ("metadata", b"{".as_slice()),
        ],
        vec![
            ("file", b"case document".as_slice()),
            (
                "metadata",
                b"{\"tags\":[],\"expected_metadata_revision\":3}".as_slice(),
            ),
        ],
    ] {
        let response = send(&workflow, multipart(&parts)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let mut incomplete = multipart(&[("file", b"case document"), ("metadata", &metadata)]);
    incomplete.truncate(incomplete.len() - BOUNDARY.len() - 8);
    assert_eq!(
        send(&workflow, incomplete).await.status(),
        StatusCode::BAD_REQUEST
    );
    for content_type in ["application/octet-stream", "multipart/form-data"] {
        let response = upload(
            &workflow,
            Some("owner-token"),
            content_type,
            Body::from("invalid"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn invalid_classification_is_rejected_before_file_preparation() {
    let workflow = Arc::new(StubWorkflow::default());
    for value in [
        json!({"tags":[""]}),
        json!({"tags":["\u{0085}value"]}),
        json!({"document_type":"x".repeat(81),"tags":[]}),
    ] {
        let response = send(
            &workflow,
            multipart(&[
                ("file", b"case document"),
                ("metadata", value.to_string().as_bytes()),
            ]),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            json_body(response).await["error"]["code"],
            "invalid_document_metadata"
        );
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn classified_upload_keeps_authentication_and_client_denial() {
    let workflow = Arc::new(StubWorkflow::default());
    let body = multipart(&[("file", b"case document"), ("metadata", &metadata())]);
    for (token, status) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some("expired-token"), StatusCode::UNAUTHORIZED),
        (Some("client-token"), StatusCode::FORBIDDEN),
    ] {
        let response = upload(
            &workflow,
            token,
            &format!("multipart/form-data; boundary={BOUNDARY}"),
            Body::from(body.clone()),
        )
        .await;
        assert_eq!(response.status(), status);
    }
}

#[tokio::test]
async fn each_multipart_part_has_an_independent_exact_size_limit() {
    let workflow = Arc::new(StubWorkflow::default());
    let mut metadata = metadata();
    metadata.resize(8192, b' ');
    let file = vec![42; FILE_LIMIT];
    let response = send(
        &workflow,
        multipart(&[("file", &file), ("metadata", &metadata)]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    metadata.push(b' ');
    let response = send(
        &workflow,
        multipart(&[("file", b"small"), ("metadata", &metadata)]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let response = send(
        &workflow,
        multipart(&[
            ("file", &vec![42; FILE_LIMIT + 1]),
            ("metadata", b"{\"tags\":[]}"),
        ]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn fragmented_uploads_need_no_content_length_and_still_obey_file_limits() {
    let workflow = Arc::new(StubWorkflow::default());
    for (file, status) in [
        (b"case document".to_vec(), StatusCode::CREATED),
        (vec![42; FILE_LIMIT + 1], StatusCode::PAYLOAD_TOO_LARGE),
    ] {
        let bytes = multipart(&[("file", &file), ("metadata", &metadata())]);
        let chunks: Vec<_> = bytes
            .chunks(65536)
            .map(|chunk| Ok::<_, std::io::Error>(Bytes::copy_from_slice(chunk)))
            .collect();
        let response = upload(
            &workflow,
            Some("owner-token"),
            &format!("multipart/form-data; boundary={BOUNDARY}"),
            Body::from_stream(futures_util::stream::iter(chunks)),
        )
        .await;
        assert_eq!(response.status(), status);
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn excessive_multipart_headers_count_towards_the_whole_body_limit() {
    let workflow = Arc::new(StubWorkflow::default());
    let mut bytes = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"\r\nX-Padding: {}\r\n\r\n",
        "x".repeat(32768)
    )
    .into_bytes();
    bytes.extend(vec![42; FILE_LIMIT]);
    bytes.extend_from_slice(b"\r\n");
    bytes.extend(multipart(&[("metadata", &metadata())]));
    let response = send(&workflow, bytes).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn bytes_after_the_multipart_terminator_still_count_towards_the_request_limit() {
    let workflow = Arc::new(StubWorkflow::default());
    let complete = multipart(&[("file", b"case document"), ("metadata", &metadata())]);
    let mut chunks = vec![Ok::<_, std::io::Error>(Bytes::from(complete))];
    chunks.extend((0..257).map(|_| Ok(Bytes::from(vec![b'x'; 65536]))));
    let response = upload(
        &workflow,
        Some("owner-token"),
        &format!("multipart/form-data; boundary={BOUNDARY}"),
        delayed_body(chunks),
    )
    .await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_failed_request_stream_cannot_commit_even_after_complete_parts() {
    let workflow = Arc::new(StubWorkflow::default());
    let complete = multipart(&[("file", b"case document"), ("metadata", &metadata())]);
    let chunks = vec![
        Ok(Bytes::from(complete)),
        Err(std::io::Error::other("injected read failure")),
    ];
    let response = upload(
        &workflow,
        Some("owner-token"),
        &format!("multipart/form-data; boundary={BOUNDARY}"),
        delayed_body(chunks),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

fn delayed_body(chunks: Vec<Result<Bytes, std::io::Error>>) -> Body {
    Body::from_stream(futures_util::stream::unfold(
        chunks.into_iter(),
        |mut chunks| async move {
            tokio::task::yield_now().await;
            chunks.next().map(|chunk| (chunk, chunks))
        },
    ))
}
