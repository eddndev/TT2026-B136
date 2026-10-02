use super::password_reset_runtime_support::*;
use std::io::{BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

#[test]
fn operating_system_reset_tokens_have_the_owned_zeroizing_32_byte_contract() {
    let token: Zeroizing<[u8; 32]> = RandomResetTokenSource
        .generate()
        .unwrap_or_else(|_| panic!("operating system entropy source failed"));
    assert_eq!(token.len(), 32);
}

#[test]
fn an_unreachable_redis_endpoint_fails_closed_without_disclosing_connection_input() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let url = format!("redis://private-reset-user:private-reset-password@{address}/7");
    let result = RedisPasswordResetLimiter::connect(
        &url,
        policy(2, 1),
        Duration::from_millis(100),
        Duration::from_millis(100),
    )
    .and_then(|store| store.admit_request("private-reset-email@example.test"));
    let error = result.expect_err("unreachable Redis must not admit or report quota");
    for value in [
        "private-reset-user",
        "private-reset-password",
        "private-reset-email",
    ] {
        assert!(
            !error.to_string().contains(value),
            "connection failure disclosed private input"
        );
    }
}

#[test]
fn established_io_timeout_preserves_authentication_and_database_selection() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (ready, observed) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(3);
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < until =>
                {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(_) => return (false, false),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut stream = BufReader::new(stream);
        let mut parser = redis::Parser::new();
        let mut authenticated = false;
        let mut selected = false;
        for _ in 0..8 {
            let Ok(value) = parser.parse_value(&mut stream) else {
                break;
            };
            let Ok(command) = redis::from_redis_value::<Vec<String>>(&value) else {
                break;
            };
            match command.first().map(String::as_str) {
                Some("AUTH") => {
                    authenticated = command == ["AUTH", "fixture-user", "fixture-password"]
                }
                Some("SELECT") => selected = command == ["SELECT", "7"],
                Some("CLIENT") => {}
                Some("EVALSHA" | "EVAL") => {
                    let _ = ready.send(());
                    let _ = released.recv_timeout(Duration::from_secs(2));
                    break;
                }
                _ => break,
            }
            if stream.get_mut().write_all(b"+OK\r\n").is_err() {
                break;
            }
        }
        (authenticated, selected)
    });
    let (sent, received) = mpsc::channel();
    let caller = std::thread::spawn(move || {
        let url = format!("redis://fixture-user:fixture-password@{address}/7");
        let result = RedisPasswordResetLimiter::connect(
            &url,
            policy(2, 1),
            Duration::from_millis(100),
            Duration::from_millis(100),
        )
        .and_then(|store| store.admit_request("timeout@example.test"));
        let _ = sent.send(result);
    });
    let command_seen = observed.recv_timeout(Duration::from_secs(3)).is_ok();
    let result = received.recv_timeout(Duration::from_secs(1));
    let _ = release.send(());
    caller.join().unwrap();
    let (authenticated, selected) = server.join().unwrap();
    assert!(
        command_seen,
        "limiter did not reach a command after the Redis handshake"
    );
    assert!(
        authenticated && selected,
        "limiter lost URL authentication or database selection"
    );
    let error = result
        .expect("established command exceeded its I/O bound")
        .expect_err("unanswered command must fail closed");
    assert!(!error.to_string().contains("fixture-password"));
    assert!(!error.to_string().contains("timeout@example.test"));
}
