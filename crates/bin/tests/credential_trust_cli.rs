use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn publication_help_requires_explicit_sources_and_expected_revision() {
    let output = Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .args(["credential-trust", "publish", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for flag in ["--root-cert", "--crl", "--expected-revision"] {
        assert!(help.contains(flag), "{flag}");
    }
}

#[test]
fn publication_bounds_public_files_before_opening_the_database() {
    for (root_size, crl_size, message) in [
        (16 * 1024 + 1, 4, "credential root exceeds 16 KiB"),
        (4, 1024 * 1024 + 1, "credential CRL exceeds 1 MiB"),
    ] {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("root.pem"), vec![b'a'; root_size]).unwrap();
        fs::write(dir.path().join("crl.pem"), vec![b'b'; crl_size]).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
            .current_dir(dir.path())
            .env(
                "DATABASE_URL",
                "postgresql://unused:secret@127.0.0.1:1/unused",
            )
            .args([
                "credential-trust",
                "publish",
                "--root-cert",
                "root.pem",
                "--crl",
                "crl.pem",
                "--expected-revision",
                "0",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains(message), "{error}");
        assert!(!error.contains("secret"));
    }
}

#[test]
fn publication_requires_the_database_environment_and_nonnegative_revision() {
    let dir = tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .current_dir(dir.path())
        .env_remove("DATABASE_URL")
        .args([
            "credential-trust",
            "publish",
            "--root-cert",
            "root.pem",
            "--crl",
            "crl.pem",
            "--expected-revision",
            "0",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("DATABASE_URL must be set for credential trust publication"));
    for revision in ["-1", "4294967296", "1.5"] {
        let output = Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
            .current_dir(dir.path())
            .args([
                "credential-trust",
                "publish",
                "--root-cert",
                "root.pem",
                "--crl",
                "crl.pem",
                "--expected-revision",
                revision,
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
}
