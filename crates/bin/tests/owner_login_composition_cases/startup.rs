use std::process::{Command, Output};

use super::options::{flags, OPTIONS};

const PRIVATE_VALUE: &str = "owner-login-connection-sentinel";

fn invocation(directory: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_despacho-cli"));
    command.current_dir(directory).env_clear().args([
        "serve",
        "--qpdf-library",
        "/nonexistent-owner-login-fixture/qpdf",
        "--signer-cert",
        "unused",
        "--signer-key",
        "unused",
    ]);
    command.env(
        "DATABASE_URL",
        format!("postgresql://{PRIVATE_VALUE}@invalid.invalid/db"),
    );
    command.env(
        "REDIS_URL",
        format!("redis://:{PRIVATE_VALUE}@invalid.invalid/"),
    );
    command
}

fn diagnostic(output: Output) -> String {
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains(PRIVATE_VALUE));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(PRIVATE_VALUE));
    String::from_utf8(output.stderr).unwrap()
}

fn native_boundary(output: Output) {
    let text = diagnostic(output);
    assert!(text.contains("cannot locate native qpdf library"), "{text}");
    assert!(!text.contains("owner certificate login settings"));
}

#[test]
fn server_resolves_opt_in_before_native_checks_connections_or_bind() {
    let directory = tempfile::tempdir().unwrap();
    // Bound dotenv discovery to the disposable fixture instead of a parent file.
    std::fs::write(directory.path().join(".env"), b"").unwrap();
    native_boundary(invocation(directory.path()).output().unwrap());
    native_boundary(
        invocation(directory.path())
            .env("TT_OWNER_LOGIN_ENABLED", "false")
            .env("TT_OWNER_LOGIN_START_GLOBAL_MAX", "0")
            .env("TT_OWNER_LOGIN_PROOF_TOKEN_WINDOW_SECONDS", "86401")
            .output()
            .unwrap(),
    );

    for (_, missing, _) in OPTIONS {
        let mut command = invocation(directory.path());
        command.env("TT_OWNER_LOGIN_ENABLED", "true");
        for (_, suffix, value) in OPTIONS {
            if suffix != missing {
                command.env(format!("TT_OWNER_LOGIN_{suffix}"), value);
            }
        }
        let text = diagnostic(command.output().unwrap());
        assert!(
            text.contains("owner certificate login settings"),
            "missing {missing}: {text}"
        );
        assert!(!text.contains("cannot locate native"));
        assert!(!text.contains("cannot initialize"));
        assert!(!text.contains("cannot bind"));
    }
    let mut environment = invocation(directory.path());
    environment.env("TT_OWNER_LOGIN_ENABLED", "true");
    for (_, suffix, value) in OPTIONS {
        environment.env(format!("TT_OWNER_LOGIN_{suffix}"), value);
    }
    native_boundary(environment.output().unwrap());
    native_boundary(
        invocation(directory.path())
            .arg("--owner-login-enabled")
            .args(flags())
            .output()
            .unwrap(),
    );
}
