use clap::{error::ErrorKind, Args, Command};
use std::ffi::OsStr;

#[allow(dead_code)]
#[path = "../src/serve_args.rs"]
mod serve_args;

fn command() -> Command {
    serve_args::ServeArgs::augment_args(Command::new("serve"))
}

fn isolated_command() -> Command {
    command().mut_args(|argument| argument.env(None::<&str>))
}

fn arguments() -> Vec<String> {
    [
        "serve",
        "--qpdf-library",
        "unused",
        "--signer-cert",
        "unused",
        "--signer-key",
        "unused",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[test]
fn serve_exposes_optional_explicit_idle_configuration() {
    let command = command();
    let argument = command
        .get_arguments()
        .find(|argument| argument.get_id() == "session_idle_seconds")
        .expect("serve must expose the optional idle duration");
    assert_eq!(argument.get_long(), Some("session-idle-seconds"));
    assert_eq!(
        argument.get_env(),
        Some(OsStr::new("TT_SESSION_IDLE_SECONDS"))
    );
    assert!(argument.get_default_values().is_empty());
    assert!(!argument.is_required_set());
}

#[test]
fn serve_defaults_to_absolute_only_without_an_idle_duration() {
    let matches = isolated_command()
        .try_get_matches_from(arguments())
        .unwrap();
    let duration = matches
        .try_get_one::<u64>("session_idle_seconds")
        .expect("the optional idle argument must exist without an implicit value");
    assert_eq!(duration, None);
}

#[test]
fn serve_accepts_explicit_idle_durations_within_the_absolute_lifetime() {
    for value in [1_u64, 60, 86_400] {
        let mut arguments = arguments();
        arguments.extend(["--session-idle-seconds".to_owned(), value.to_string()]);
        let matches = isolated_command().try_get_matches_from(arguments).unwrap();
        assert_eq!(matches.get_one::<u64>("session_idle_seconds"), Some(&value));
    }
}

#[test]
fn serve_rejects_invalid_idle_configuration_during_argument_parsing() {
    for value in [
        "0",
        "86401",
        "18446744073709551616",
        "-1",
        "1.5",
        "disabled",
    ] {
        let option = format!("--session-idle-seconds={value}");
        let mut arguments = arguments();
        arguments.push(option);
        let error = isolated_command()
            .try_get_matches_from(arguments)
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::ValueValidation, "{value}: {error}");
        assert!(error.to_string().contains("--session-idle-seconds"));
    }
}
