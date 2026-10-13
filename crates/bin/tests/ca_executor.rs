#[path = "../src/ca/executor.rs"]
mod executor;

use application::{
    ca_access::OperatorIdentity,
    ca_operations::{CaCommand, CaExecutor, CaPurpose},
    ApplicationError, PortFailureKind,
};
use domain::identity::UserId;
use executor::NativeCaExecutor;
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

struct Fixture {
    temporary: tempfile::TempDir,
    worker: PathBuf,
    python: PathBuf,
}

impl Fixture {
    fn new(body: &str) -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let worker = temporary.path().join("worker.py");
        fs::write(&worker, body).unwrap();
        Self {
            temporary,
            worker,
            python: PathBuf::from("/usr/bin/python3"),
        }
    }

    fn executor(&self) -> NativeCaExecutor {
        NativeCaExecutor::new(
            self.python.clone(),
            self.worker.clone(),
            self.temporary.path().to_path_buf(),
            self.python.clone(),
            self.python.clone(),
            self.temporary.path().to_path_buf(),
            "a".repeat(64),
        )
        .unwrap()
        .with_timeout(Duration::from_secs(3))
    }

    fn actor() -> OperatorIdentity {
        OperatorIdentity::new(
            UserId::from_uuid("20000000-0000-4000-8000-000000000001".parse().unwrap()),
            "private-email-not-needed@example.test",
        )
        .unwrap()
    }
}

fn unavailable(error: &ApplicationError) {
    assert!(matches!(
        error,
        ApplicationError::ClassifiedPort {
            kind: PortFailureKind::Unavailable,
            ..
        }
    ));
    let message = error.to_string();
    assert!(!message.contains("PROVIDER_SECRET"));
    assert!(!message.contains("private-email"));
}

#[test]
fn authenticated_actor_and_typed_command_are_the_only_input_with_fixed_configured_arguments() {
    let fixture = Fixture::new(
        r#"
import json, os, sys
request = json.load(sys.stdin)
print(json.dumps({'ok': True, 'value': {
    'input': request, 'arguments': sys.argv[1:],
    'flags': [sys.flags.isolated, sys.flags.no_site, sys.flags.dont_write_bytecode],
    'environment': dict(os.environ)}}))
"#,
    );
    let result = fixture
        .executor()
        .execute(
            &Fixture::actor(),
            CaCommand::Inventory {
                offset: 8,
                limit: 25,
            },
        )
        .unwrap();
    assert_eq!(
        result["input"],
        json!({
            "operator_id": Fixture::actor().id, "command": {"action": "inventory", "offset": 8, "limit": 25}
        })
    );
    assert_eq!(
        result["arguments"],
        json!([
            "--root",
            fixture.temporary.path(),
            "--inspector",
            fixture.python.canonicalize().unwrap(),
            "--cli",
            fixture.python.canonicalize().unwrap(),
            "--scripts",
            fixture.temporary.path(),
            "--expected-root",
            "a".repeat(64)
        ])
    );
    assert_eq!(result["flags"], json!([1, 1, 1]));
    let uid = rustix::process::geteuid().as_raw();
    assert_eq!(
        result["environment"]["XDG_RUNTIME_DIR"],
        format!("/run/user/{uid}")
    );
    assert_eq!(
        result["environment"]["DBUS_SESSION_BUS_ADDRESS"],
        format!("unix:path=/run/user/{uid}/bus")
    );
    assert!(result["environment"]
        .as_object()
        .unwrap()
        .keys()
        .all(|key| [
            "PATH",
            "LC_ALL",
            "TZ",
            "LC_CTYPE",
            "XDG_RUNTIME_DIR",
            "DBUS_SESSION_BUS_ADDRESS"
        ]
        .contains(&key.as_str())));
    assert!(!result.to_string().contains("private-email"));
}

#[test]
fn known_worker_failures_keep_categories_without_provider_details() {
    for code in ["invalid", "busy", "unavailable"] {
        let fixture = Fixture::new(&format!(
            "import sys\nsys.stdin.read()\nsys.stderr.write('PROVIDER_SECRET')\nprint('{{\"ok\":false,\"code\":\"{code}\"}}')\n"
        ));
        let error = fixture
            .executor()
            .execute(&Fixture::actor(), CaCommand::Status {})
            .unwrap_err();
        match code {
            "invalid" => assert!(matches!(error, ApplicationError::InvalidInput(_))),
            "busy" => assert!(matches!(
                error,
                ApplicationError::ClassifiedPort {
                    kind: PortFailureKind::Busy,
                    ..
                }
            )),
            _ => unavailable(&error),
        }
        assert!(!error.to_string().contains("PROVIDER_SECRET"));
    }
}

#[test]
fn malformed_unknown_or_nonzero_worker_results_fail_closed_with_generic_errors() {
    for response in [
        "not JSON PROVIDER_SECRET",
        "{}",
        "[]",
        "{\"ok\":true}",
        "{\"ok\":false,\"code\":\"PROVIDER_SECRET\"}",
        "{\"ok\":true,\"value\":1,\"extra\":\"PROVIDER_SECRET\"}",
        "{\"ok\":true,\"ok\":false,\"value\":1}",
        "{\"ok\":false,\"value\":1}",
        "{\"ok\":true,\"code\":\"busy\"}",
    ] {
        let fixture = Fixture::new(&format!(
            "print({})\n",
            serde_json::to_string(response).unwrap()
        ));
        unavailable(
            &fixture
                .executor()
                .execute(&Fixture::actor(), CaCommand::Status {})
                .unwrap_err(),
        );
    }
    let fixture = Fixture::new("import sys\nprint('{\"ok\":true,\"value\":1}')\nsys.stderr.write('PROVIDER_SECRET')\nsys.exit(12)\n");
    unavailable(
        &fixture
            .executor()
            .execute(&Fixture::actor(), CaCommand::Status {})
            .unwrap_err(),
    );
}

#[test]
fn success_supports_null_and_bounded_json_values() {
    let fixture =
        Fixture::new("import sys\nsys.stdin.read()\nprint('{\"ok\":true,\"value\":null}')\n");
    assert_eq!(
        fixture
            .executor()
            .execute(&Fixture::actor(), CaCommand::Status {})
            .unwrap(),
        json!(null)
    );
}

#[test]
fn oversized_input_and_nil_actor_are_rejected_before_starting_worker() {
    let fixture =
        Fixture::new("import pathlib\npathlib.Path(__file__).with_suffix('.started').touch()\n");
    for value in ["x".repeat(65537), "\"".repeat(33000)] {
        let error = fixture
            .executor()
            .execute(
                &Fixture::actor(),
                CaCommand::Preview {
                    csr_pem: value,
                    purpose: CaPurpose::Partner,
                },
            )
            .unwrap_err();
        assert!(matches!(error, ApplicationError::InvalidInput(_)));
    }
    let nil = OperatorIdentity {
        id: UserId::from_uuid(uuid::Uuid::nil()),
        email: "a@example.test".into(),
    };
    assert!(fixture
        .executor()
        .execute(&nil, CaCommand::Status {})
        .is_err());
    assert!(!fixture.worker.with_extension("started").exists());
}

#[test]
fn oversized_stdout_is_bounded_while_worker_is_still_writing() {
    let fixture = Fixture::new("import os\nwhile True: os.write(1, b'x' * 65536)\n");
    let started = Instant::now();
    unavailable(
        &fixture
            .executor()
            .execute(&Fixture::actor(), CaCommand::Status {})
            .unwrap_err(),
    );
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn timeout_reaps_owned_leader_and_stops_its_descendant_without_blocking_on_stdin() {
    let fixture = Fixture::new(
        r#"
import json, os, pathlib, time
child = os.fork()
if child == 0:
    time.sleep(30)
else:
    pathlib.Path(__file__).with_suffix('.pids').write_text(json.dumps([os.getpid(), child]))
    time.sleep(30)
"#,
    );
    let started = Instant::now();
    let executor = fixture.executor().with_timeout(Duration::from_millis(750));
    unavailable(
        &executor
            .execute(
                &Fixture::actor(),
                CaCommand::Preview {
                    csr_pem: "x".repeat(60000),
                    purpose: CaPurpose::Partner,
                },
            )
            .unwrap_err(),
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    let pids: Vec<u32> =
        serde_json::from_slice(&fs::read(fixture.worker.with_extension("pids")).unwrap()).unwrap();
    assert!(
        !PathBuf::from(format!("/proc/{}", pids[0])).exists(),
        "owned leader was not reaped"
    );
    // Waiting for the leader does not wait for SIGKILL in other group members.
    let until = Instant::now() + Duration::from_secs(1);
    loop {
        match fs::read_to_string(format!("/proc/{}/stat", pids[1])) {
            Ok(status) => {
                let state = status.rsplit_once(") ").unwrap().1.as_bytes()[0];
                if matches!(state, b'Z' | b'X') {
                    break;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => panic!("cannot observe owned descendant: {error}"),
        }
        assert!(Instant::now() < until, "owned descendant is still running");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn construction_rejects_unconfigured_paths_and_noncanonical_root_identity() {
    let fixture = Fixture::new("print('{\"ok\":true,\"value\":null}')\n");
    for expected in ["a".repeat(63), "A".repeat(64), "../root".into()] {
        assert!(NativeCaExecutor::new(
            fixture.python.clone(),
            fixture.worker.clone(),
            fixture.temporary.path().into(),
            fixture.python.clone(),
            fixture.python.clone(),
            fixture.temporary.path().into(),
            expected
        )
        .is_err());
    }
    assert!(NativeCaExecutor::new(
        PathBuf::from("python3"),
        fixture.worker.clone(),
        fixture.temporary.path().into(),
        fixture.python.clone(),
        fixture.python.clone(),
        fixture.temporary.path().into(),
        "a".repeat(64)
    )
    .is_err());
}
