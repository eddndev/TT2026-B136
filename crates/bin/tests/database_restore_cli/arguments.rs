use crate::password_reset_restore_support::PasswordResetRestoreRequest;
use crate::support::*;

fn request() -> PasswordResetRestoreRequest {
    PasswordResetRestoreRequest {
        operation_id: uuid::Uuid::new_v4(),
        expected_database: "restore_fixture".into(),
        expected_schema: "application".into(),
        expected_head: None,
    }
}

#[test]
fn administrative_help_exposes_explicit_identity_and_no_credential_flags() {
    for subcommand in [INVALIDATE, "check"] {
        let output = invoke(None, &args(&["database", subcommand, "--help"]));
        assert!(
            output.status.success(),
            "administrative command is unavailable"
        );
        let help = String::from_utf8(output.stdout).unwrap();
        for forbidden in [
            "--database-url",
            "--password",
            "--admin-url",
            "--runtime-role",
        ] {
            assert!(
                !help.contains(forbidden),
                "unexpected credential or mutation flag"
            );
        }
        if subcommand == INVALIDATE {
            for required in [
                "--operation-id",
                "--expected-database",
                "--expected-schema",
                "--expected-empty-audit",
                "--expected-audit-sequence",
                "--expected-audit-head",
            ] {
                assert!(
                    help.contains(required),
                    "missing explicit restore field {required}"
                );
            }
        }
    }
}

#[test]
fn invalidation_rejects_missing_ambiguous_and_noncanonical_fields_before_connection() {
    let valid = restore_args(&request());
    let mut invalid = Vec::new();
    for flag in [
        "--operation-id",
        "--expected-database",
        "--expected-schema",
        "--expected-empty-audit",
    ] {
        let mut missing = valid.clone();
        let index = missing.iter().position(|value| value == flag).unwrap();
        missing.remove(index);
        if flag != "--expected-empty-audit" {
            missing.remove(index);
        }
        invalid.push(missing);
    }
    for (flag, values) in [
        (
            "--operation-id",
            vec![
                "00000000-0000-0000-0000-000000000000".to_owned(),
                "0123456789ab4def8123456789abcdef".into(),
                "01234567-89AB-4DEF-8123-456789ABCDEF".into(),
            ],
        ),
        ("--expected-database", vec![String::new()]),
        ("--expected-schema", vec![String::new()]),
    ] {
        for value in values {
            let mut changed = valid.clone();
            let index = changed.iter().position(|item| item == flag).unwrap();
            changed[index + 1] = value;
            invalid.push(changed);
        }
    }
    let prefix = &valid[..valid.len() - 1];
    let canonical_head = "ab".repeat(32);
    for (sequence, head) in [
        (Some("0"), None),
        (None, Some(canonical_head.as_str())),
        (Some("-1"), Some(canonical_head.as_str())),
        (Some("9223372036854775808"), Some(canonical_head.as_str())),
        (Some("0"), Some("")),
        (Some("0"), Some("xyz")),
    ] {
        let mut changed = prefix.to_vec();
        if let Some(value) = sequence {
            changed.extend(args(&["--expected-audit-sequence", value]));
        }
        if let Some(value) = head {
            changed.extend(args(&["--expected-audit-head", value]));
        }
        invalid.push(changed);
    }
    for head in ["AB".repeat(32), "aa".repeat(31), "aa".repeat(33)] {
        let mut changed = prefix.to_vec();
        changed.extend(args(&[
            "--expected-audit-sequence",
            "0",
            "--expected-audit-head",
            &head,
        ]));
        invalid.push(changed);
    }
    let mut ambiguous = valid;
    ambiguous.extend(args(&[
        "--expected-audit-sequence",
        "0",
        "--expected-audit-head",
        &"ab".repeat(32),
    ]));
    invalid.push(ambiguous);
    for (index, arguments) in invalid.iter().enumerate() {
        let output = invoke(None, arguments);
        assert_eq!(
            output.status.code(),
            Some(2),
            "invalid argument set {index}"
        );
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn both_commands_require_environment_connection_and_neutral_connection_errors() {
    let mut nonempty = request();
    nonempty.expected_head = Some(
        crate::password_reset_restore_support::PasswordResetRestoreHead {
            sequence: 0,
            chain: crate::password_reset_backend_support::digest(10),
        },
    );
    let commands = [
        restore_args(&request()),
        restore_args(&nonempty),
        args(&["--json", "database", "check"]),
    ];
    for arguments in commands {
        let missing = invoke(None, &arguments);
        assert_eq!(missing.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&missing.stderr).contains("DATABASE_URL"));
        assert!(missing.stdout.is_empty());
        let bad_url =
            format!("postgresql://fixture:{PRIVATE_SENTINEL}@127.0.0.1:1/unused?connect_timeout=1");
        let unavailable = invoke(Some(&bad_url), &arguments);
        assert_eq!(unavailable.status.code(), Some(1));
        rejected(unavailable);
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_database_environment_is_rejected_without_disclosing_its_value() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::process::Command;

    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join(".env"), b"").unwrap();
    let mut bytes = PRIVATE_SENTINEL.as_bytes().to_vec();
    bytes.push(0xff);
    let private = OsString::from_vec(bytes);
    let observations: Vec<_> = [
        args(&["--json", "database", "check"]),
        restore_args(&request()),
    ]
    .into_iter()
    .map(|arguments| {
        let output = Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
            .current_dir(directory.path())
            .env_clear()
            .env("RUST_LOG", "off")
            .env("DATABASE_URL", &private)
            .args(arguments)
            .output()
            .expect("the CLI starts");
        let leaked = [&output.stdout, &output.stderr]
            .into_iter()
            .any(|part| String::from_utf8_lossy(part).contains(PRIVATE_SENTINEL));
        (
            output.status.code(),
            output.stdout.is_empty(),
            output.stderr.is_empty(),
            leaked,
        )
    })
    .collect();
    // Inspect both commands before asserting, without printing their private output.
    assert!(
        observations
            .iter()
            .all(|entry| *entry == (Some(1), true, false, false)),
        "non-UTF-8 connection input must fail neutrally in both commands"
    );
}
