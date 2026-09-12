use std::io::{BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

use application::identity::SessionStore;
use application::ApplicationError;
use infrastructure::RedisSessionStore;

#[test]
fn an_established_connection_times_out_when_a_command_never_receives_a_reply() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (command_sent, command_received) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut stream = BufReader::new(stream);
        let mut parser = redis::Parser::new();
        for _ in 0..2 {
            parser.parse_value(&mut stream).unwrap();
            stream.get_mut().write_all(b"+OK\r\n").unwrap();
        }
        parser.parse_value(&mut stream).unwrap();
        command_sent.send(()).unwrap();
        let _ = released.recv_timeout(Duration::from_secs(2));
    });
    let store = RedisSessionStore::connect_with_timeouts(
        &format!("redis://{address}/"),
        Duration::from_millis(100),
        Duration::from_millis(100),
    )
    .unwrap();
    let (result_sent, result_received) = mpsc::channel();
    let caller = std::thread::spawn(move || {
        result_sent
            .send(store.find_session("opaque-token"))
            .unwrap();
    });
    command_received
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    let result = result_received.recv_timeout(Duration::from_secs(1));
    release.send(()).unwrap();
    caller.join().unwrap();
    server.join().unwrap();
    assert!(matches!(result, Ok(Err(ApplicationError::Port(_)))));
}

#[test]
fn zero_connection_or_io_timeouts_are_rejected() {
    for (connect, io) in [
        (Duration::ZERO, Duration::from_secs(1)),
        (Duration::from_secs(1), Duration::ZERO),
    ] {
        assert!(matches!(
            RedisSessionStore::connect_with_timeouts("redis://127.0.0.1/", connect, io),
            Err(ApplicationError::InvalidConfiguration(_))
        ));
    }
}
