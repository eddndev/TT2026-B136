//! Contract tests of the Cincel timestamping adapter against a local
//! stub HTTP server, covering the immediate-token path, the deferred
//! processing path, rejections, unreachable hosts, and the promise that
//! the api key never leaks into errors.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use domain::crypto::timestamp::TimestampService;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::DomainError;
use infrastructure::timestamp::{CincelTsaAdapter, RetryPolicy};
use infrastructure::RingSha256Hasher;
use zeroize::Zeroizing;

/// One HTTP request as the stub server saw it.
struct SeenRequest {
    request_line: String,
    /// Header names lower-cased, values as sent.
    headers: Vec<(String, String)>,
    body: String,
}

impl SeenRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Serves the given responses to consecutive connections on a random
/// local port and reports each request it saw through the receiver.
fn stub_server(responses: Vec<String>) -> (String, Receiver<SeenRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port is available");
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let (sender, receiver) = channel();
    thread::spawn(move || {
        for response in responses {
            let (stream, _) = listener.accept().expect("a client connects");
            let mut reader = BufReader::new(stream);
            let seen = read_request(&mut reader);
            let _ = sender.send(seen);
            let _ = reader.get_mut().write_all(response.as_bytes());
        }
    });
    (base_url, receiver)
}

fn read_request(reader: &mut BufReader<std::net::TcpStream>) -> SeenRequest {
    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();
    let mut headers = Vec::new();
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let line = line.trim_end().to_string();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            if name == "content-length" {
                content_length = value.parse().unwrap();
            }
            headers.push((name, value));
        }
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).unwrap();
    SeenRequest {
        request_line: request_line.trim_end().to_string(),
        headers,
        body: String::from_utf8(body).unwrap(),
    }
}

fn json_response(status_line: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn adapter_for(base_url: &str, api_key: &str) -> CincelTsaAdapter {
    CincelTsaAdapter::new(
        base_url,
        Zeroizing::new(api_key.to_string()),
        Duration::from_secs(2),
        RetryPolicy {
            max_attempts: 3,
            delay: Duration::from_millis(10),
        },
    )
    .expect("the http client builds")
}

fn sample_digest() -> Sha256Digest {
    RingSha256Hasher::new().hash_bytes(b"contrato para sellado remoto")
}

fn failure_message(err: DomainError) -> String {
    match err {
        DomainError::TimestampAuthorityFailure(message) => message,
        other => panic!("expected a timestamp authority failure, got {other:?}"),
    }
}

#[test]
fn an_immediately_completed_submission_returns_the_decoded_token() {
    let token_der = b"fake-der-token-bytes".to_vec();
    let body = format!(
        r#"{{"status":"completed","token_base64":"{}"}}"#,
        BASE64.encode(&token_der)
    );
    let (base_url, seen) = stub_server(vec![json_response("200 OK", &body)]);

    let digest = sample_digest();
    let token = adapter_for(&base_url, "clave-de-api")
        .request(&digest)
        .unwrap();
    assert_eq!(token, token_der);

    let submit = seen.recv().unwrap();
    assert_eq!(submit.request_line, "POST /api/v1/timestamps HTTP/1.1");
    let sent: serde_json::Value = serde_json::from_str(&submit.body).unwrap();
    assert_eq!(sent["digest_algorithm"], "sha-256");
    assert_eq!(sent["digest_hex"], digest.to_hex());
}

#[test]
fn a_processing_submission_is_polled_until_the_token_arrives() {
    let token_der = b"deferred-token".to_vec();
    let completed = format!(
        r#"{{"status":"completed","token_base64":"{}"}}"#,
        BASE64.encode(&token_der)
    );
    let (base_url, seen) = stub_server(vec![
        json_response("202 Accepted", r#"{"status":"processing","id":"job-7"}"#),
        json_response("200 OK", r#"{"status":"processing","id":"job-7"}"#),
        json_response("200 OK", &completed),
    ]);

    let token = adapter_for(&base_url, "clave-de-api")
        .request(&sample_digest())
        .unwrap();
    assert_eq!(token, token_der);

    let submit = seen.recv().unwrap();
    assert_eq!(submit.request_line, "POST /api/v1/timestamps HTTP/1.1");
    let first_poll = seen.recv().unwrap();
    assert_eq!(
        first_poll.request_line,
        "GET /api/v1/timestamps/job-7 HTTP/1.1"
    );
    let second_poll = seen.recv().unwrap();
    assert_eq!(
        second_poll.request_line,
        "GET /api/v1/timestamps/job-7 HTTP/1.1"
    );
}

#[test]
fn a_submission_still_processing_after_the_retry_budget_fails() {
    let processing = json_response("200 OK", r#"{"status":"processing","id":"job-9"}"#);
    let (base_url, _seen) = stub_server(vec![
        processing.clone(),
        processing.clone(),
        processing.clone(),
        processing,
    ]);

    let adapter = CincelTsaAdapter::new(
        &base_url,
        Zeroizing::new("clave-de-api".to_string()),
        Duration::from_secs(2),
        RetryPolicy {
            max_attempts: 3,
            delay: Duration::from_millis(10),
        },
    )
    .unwrap();
    let message = failure_message(adapter.request(&sample_digest()).unwrap_err());
    assert!(
        message.contains("still processing"),
        "the exhausted retry budget should be named, got: {message}"
    );
}

#[test]
fn a_provider_rejection_carries_its_detail() {
    let (base_url, _seen) = stub_server(vec![json_response(
        "200 OK",
        r#"{"status":"rejected","detail":"unsupported digest"}"#,
    )]);

    let message = failure_message(
        adapter_for(&base_url, "clave-de-api")
            .request(&sample_digest())
            .unwrap_err(),
    );
    assert!(
        message.contains("rejected") && message.contains("unsupported digest"),
        "the provider detail should be kept, got: {message}"
    );
}

#[test]
fn an_http_error_status_is_a_rejection_naming_the_status() {
    let (base_url, _seen) = stub_server(vec![json_response(
        "403 Forbidden",
        r#"{"message":"bad credentials"}"#,
    )]);

    let message = failure_message(
        adapter_for(&base_url, "clave-de-api")
            .request(&sample_digest())
            .unwrap_err(),
    );
    assert!(
        message.contains("403"),
        "the http status should be named, got: {message}"
    );
}

#[test]
fn an_undecodable_body_is_an_invalid_token() {
    let (base_url, _seen) = stub_server(vec![json_response("200 OK", "this is not json")]);

    let message = failure_message(
        adapter_for(&base_url, "clave-de-api")
            .request(&sample_digest())
            .unwrap_err(),
    );
    assert!(
        message.contains("invalid timestamp token"),
        "an unusable body should map to the invalid token cause, got: {message}"
    );
}

#[test]
fn a_connection_closed_without_a_response_is_an_unreachable_authority() {
    // Keep the port reserved until the request arrives. Dropping a listener
    // before connecting lets another concurrent stub acquire its address.
    let (base_url, seen) = stub_server(vec![String::new()]);

    let message = failure_message(
        adapter_for(&base_url, "clave-de-api")
            .request(&sample_digest())
            .unwrap_err(),
    );
    assert!(
        message.contains("unreachable"),
        "a transport failure should map to the unreachable cause, got: {message}"
    );
    assert_eq!(
        seen.recv().unwrap().request_line,
        "POST /api/v1/timestamps HTTP/1.1"
    );
}

#[test]
fn the_api_key_travels_in_the_header_and_never_into_errors() {
    let secret = "clave-secreta-que-no-debe-filtrarse";

    let (ok_url, seen) = stub_server(vec![json_response(
        "200 OK",
        &format!(
            r#"{{"status":"completed","token_base64":"{}"}}"#,
            BASE64.encode(b"token")
        ),
    )]);
    adapter_for(&ok_url, secret)
        .request(&sample_digest())
        .unwrap();
    let submit = seen.recv().unwrap();
    assert_eq!(submit.header("x-api-key"), Some(secret));

    let (bad_url, _seen) = stub_server(vec![json_response(
        "500 Internal Server Error",
        "backend exploded",
    )]);
    let adapter = adapter_for(&bad_url, secret);
    assert!(
        !format!("{adapter:?}").contains(secret),
        "the adapter debug form must redact the api key"
    );
    let err = adapter.request(&sample_digest()).unwrap_err();
    assert!(
        !err.to_string().contains(secret) && !format!("{err:?}").contains(secret),
        "no error may carry the api key"
    );
}

/// Serves `body` as a single 200 response and returns the domain-level
/// failure message the adapter raises for it.
fn failure_for_body(body: &str) -> String {
    let (base_url, _seen) = stub_server(vec![json_response("200 OK", body)]);
    failure_message(
        adapter_for(&base_url, "clave-de-api")
            .request(&sample_digest())
            .unwrap_err(),
    )
}

#[test]
fn a_completed_response_without_a_token_is_an_invalid_token() {
    let message = failure_for_body(r#"{"status":"completed"}"#);
    assert!(
        message.contains("carries no token"),
        "a completed response must name the missing token, got: {message}"
    );
}

#[test]
fn a_token_that_is_not_base64_is_an_invalid_token() {
    let message = failure_for_body(r#"{"status":"completed","token_base64":"not base64 !!!"}"#);
    assert!(
        message.contains("base64"),
        "an undecodable token must name the encoding, got: {message}"
    );
}

#[test]
fn a_completed_response_carrying_an_empty_token_is_rejected() {
    let message = failure_for_body(r#"{"status":"completed","token_base64":""}"#);
    assert!(
        message.contains("empty"),
        "a decoded empty token must be rejected as empty, got: {message}"
    );
}

#[test]
fn a_processing_response_without_an_id_is_an_invalid_token() {
    let message = failure_for_body(r#"{"status":"processing"}"#);
    assert!(
        message.contains("no id"),
        "a processing response must name the missing id, got: {message}"
    );
}

#[test]
fn an_unrecognized_status_is_an_invalid_token() {
    let message = failure_for_body(r#"{"status":"minted-somehow"}"#);
    assert!(
        message.contains("unknown provider status"),
        "an unrecognized status must be reported, got: {message}"
    );
}

/// Smoke test against the provider's real sandbox. Ignored because it
/// needs live credentials: run it manually with CINCEL_BASE_URL and
/// CINCEL_API_KEY set in the environment.
#[test]
#[ignore]
fn the_real_sandbox_issues_a_token() {
    let base_url = std::env::var("CINCEL_BASE_URL").expect("CINCEL_BASE_URL is set");
    let api_key = Zeroizing::new(std::env::var("CINCEL_API_KEY").expect("CINCEL_API_KEY is set"));
    let adapter = CincelTsaAdapter::new(
        &base_url,
        api_key,
        Duration::from_secs(30),
        RetryPolicy {
            max_attempts: 10,
            delay: Duration::from_secs(3),
        },
    )
    .unwrap();

    let token = adapter.request(&sample_digest()).unwrap();
    assert!(!token.is_empty());
}
