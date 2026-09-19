use super::*;
use application::alerts::AlertEmailTemplate;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Instant;

struct Server {
    endpoint: String,
    stop: Arc<AtomicBool>,
    task: Option<JoinHandle<Vec<u8>>>,
}

impl Server {
    fn new(status: u16, body: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}/emails", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let body = body.to_owned();
        let task = thread::spawn(move || {
            let end = Instant::now() + Duration::from_secs(2);
            while Instant::now() < end && !stopped.load(Ordering::Relaxed) {
                if let Ok((mut stream, _)) = listener.accept() {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(1)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut buf = [0; 2048];
                    loop {
                        let n = stream.read(&mut buf).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        request.extend_from_slice(&buf[..n]);
                        if let Some(split) = request.windows(4).position(|part| part == b"\r\n\r\n")
                        {
                            let header = String::from_utf8_lossy(&request[..split]);
                            let length = header
                                .lines()
                                .find_map(|line| {
                                    line.to_ascii_lowercase()
                                        .strip_prefix("content-length: ")
                                        .map(str::to_owned)
                                })
                                .unwrap()
                                .parse::<usize>()
                                .unwrap();
                            if request.len() >= split + 4 + length {
                                break;
                            }
                        }
                    }
                    if status != 0 {
                        let response = format!("HTTP/1.1 {status} response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                        stream.write_all(response.as_bytes()).unwrap();
                    }
                    return request;
                }
                thread::sleep(Duration::from_millis(2));
            }
            Vec::new()
        });
        Self {
            endpoint,
            stop,
            task: Some(task),
        }
    }

    fn sender(&self) -> ResendAlertEmailSender {
        let mut sender = ResendAlertEmailSender::new("local-test-key".into()).unwrap();
        sender.endpoint = self.endpoint.clone();
        sender
    }

    fn request(&mut self) -> String {
        String::from_utf8(self.task.take().unwrap().join().unwrap()).unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(task) = self.task.take() {
            task.join().unwrap();
        }
    }
}

fn message() -> AlertEmailMessage {
    AlertEmailMessage {
        idempotency_key: "alert/00000000-0000-4000-8000-000000000001".into(),
        from_email: "avisos@example.test".into(),
        recipient_email: "member@example.test".into(),
        login_url: "https://qadra.example.test/login".into(),
        template: AlertEmailTemplate::GenericLoginV1,
    }
}

#[test]
fn accepted_submission_uses_the_frozen_key_and_only_generic_content() {
    let id = "00000000-0000-4000-8000-000000000002";
    let mut server = Server::new(200, &format!("{{\"id\":\"{id}\"}}"));
    assert_eq!(
        server.sender().send(&message()),
        AlertEmailOutcome::Accepted {
            provider_id: id.into()
        }
    );
    let request = server.request();
    let (headers, body) = request.split_once("\r\n\r\n").unwrap();
    assert!(headers
        .to_ascii_lowercase()
        .contains("idempotency-key: alert/00000000-0000-4000-8000-000000000001"));
    let body: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "from": "avisos@example.test", "to": ["member@example.test"],
            "subject": "Qadra: tienes avisos pendientes",
            "text": "Hay avisos en Qadra. Inicia sesion para consultarlos.\n\nhttps://qadra.example.test/login"
        })
    );
}

#[test]
fn lost_reply_and_invalid_success_preserve_uncertainty() {
    for (status, body) in [
        (0, ""),
        (200, "not-json"),
        (200, "{}"),
        (200, "{\"id\":\"invalid\"}"),
    ] {
        let server = Server::new(status, body);
        assert!(matches!(
            server.sender().send(&message()),
            AlertEmailOutcome::Unknown { .. }
        ));
    }
}

#[test]
fn rate_limit_and_concurrent_idempotent_request_can_retry_with_the_same_message() {
    for (status, body) in [
        (429, "{}"),
        (409, "{\"name\":\"concurrent_idempotent_requests\"}"),
    ] {
        let server = Server::new(status, body);
        assert!(matches!(
            server.sender().send(&message()),
            AlertEmailOutcome::Retryable { .. }
        ));
    }
}

#[test]
fn payload_conflict_and_credential_rejection_are_permanent() {
    for (status, body) in [
        (409, "{\"name\":\"invalid_idempotent_request\"}"),
        (401, "{}"),
        (422, "{}"),
    ] {
        let server = Server::new(status, body);
        assert!(matches!(
            server.sender().send(&message()),
            AlertEmailOutcome::Permanent { .. }
        ));
    }
}

#[test]
fn server_errors_and_redirects_never_imply_the_message_was_not_accepted() {
    for status in [500, 503, 307] {
        let server = Server::new(status, "{}");
        assert!(matches!(
            server.sender().send(&message()),
            AlertEmailOutcome::Unknown { .. }
        ));
    }
}

#[test]
fn malformed_addresses_keys_or_resource_urls_are_rejected_before_network_io() {
    for (field, value) in [
        (
            "recipient",
            "person@example.test\r\nBcc: stranger@example.test",
        ),
        ("from", ""),
        ("key", ""),
        ("key", "two\nlines"),
        ("url", "https://qadra.example.test/cases/private-id"),
        ("url", "https://qadra.example.test/login?token=secret"),
        ("url", "http://public.example.test/login"),
    ] {
        let server = Server::new(200, "{}");
        let mut item = message();
        match field {
            "recipient" => item.recipient_email = value.into(),
            "from" => item.from_email = value.into(),
            "key" => item.idempotency_key = value.into(),
            _ => item.login_url = value.into(),
        }
        assert!(matches!(
            server.sender().send(&item),
            AlertEmailOutcome::Permanent { .. }
        ));
    }
}
