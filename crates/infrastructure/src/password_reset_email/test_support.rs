use application::identity::password_reset::ResetEnvelope;
use domain::clock::OffsetDateTime;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

pub const KEY: &str = "local-reset-api-key-marker";
pub const TOKEN: [u8; 32] = [0xa5; 32];
pub const RECEIPT: &str = "{\"id\":\"0e6d1d44-17c3-4d1b-b32e-d8ffcf91681e\"}";

pub fn envelope() -> ResetEnvelope {
    ResetEnvelope {
        email: "recipient@example.test".into(),
        token: Zeroizing::new(TOKEN),
        expires_at: OffsetDateTime::from_unix_timestamp(1_893_456_000).unwrap(),
    }
}

pub enum Response {
    Reply {
        status: u16,
        body: String,
        headers: Vec<(String, String)>,
        declared: Option<usize>,
    },
    Disconnect,
    Stall,
}

impl Response {
    pub fn reply(status: u16, body: impl Into<String>) -> Self {
        Self::Reply {
            status,
            body: body.into(),
            headers: Vec::new(),
            declared: None,
        }
    }
}

pub struct Request {
    pub method: String,
    pub target: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

pub struct Server {
    pub endpoint: String,
    stop: Arc<AtomicBool>,
    task: Option<JoinHandle<()>>,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl Server {
    pub fn new(response: Response) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}/emails", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        let task = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(15);
            while !stopped.load(Ordering::Relaxed) && Instant::now() < deadline {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let Some(request) = read_request(&mut stream) else {
                    continue;
                };
                captured.lock().unwrap().push(request);
                match &response {
                    Response::Disconnect => (),
                    Response::Stall => {
                        while !stopped.load(Ordering::Relaxed) && Instant::now() < deadline {
                            thread::sleep(Duration::from_millis(2));
                        }
                    }
                    Response::Reply {
                        status,
                        body,
                        headers,
                        declared,
                    } => {
                        let mut reply = format!(
                            "HTTP/1.1 {status} response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
                            declared.unwrap_or(body.len())
                        );
                        for (name, value) in headers {
                            reply.push_str(&format!("{name}: {value}\r\n"));
                        }
                        reply.push_str("\r\n");
                        reply.push_str(body);
                        let _ = stream.write_all(reply.as_bytes());
                    }
                }
            }
        });
        Self {
            endpoint,
            stop,
            task: Some(task),
            requests,
        }
    }

    pub fn finish(mut self) -> Vec<Request> {
        self.shutdown();
        let requests = std::mem::take(&mut *self.requests.lock().unwrap());
        requests
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(task) = self.task.take() {
            task.join().expect("local HTTP fixture panicked");
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 2048];
    loop {
        let size = stream.read(&mut chunk).ok()?;
        if size == 0 {
            return None;
        }
        bytes.extend_from_slice(&chunk[..size]);
        if bytes.len() > 16 * 1024 {
            return None;
        }
        let Some(split) = bytes.windows(4).position(|part| part == b"\r\n\r\n") else {
            continue;
        };
        let text = std::str::from_utf8(&bytes[..split]).ok()?;
        let mut lines = text.lines();
        let mut first = lines.next()?.split_whitespace();
        let method = first.next()?.to_owned();
        let target = first.next()?.to_owned();
        let headers: BTreeMap<_, _> = lines
            .filter_map(|line| {
                line.split_once(':')
                    .map(|(key, value)| (key.to_ascii_lowercase(), value.trim().to_owned()))
            })
            .collect();
        let length = headers.get("content-length")?.parse::<usize>().ok()?;
        if bytes.len() < split + 4 + length {
            continue;
        }
        return Some(Request {
            method,
            target,
            headers,
            body: bytes[split + 4..split + 4 + length].to_vec(),
        });
    }
}
