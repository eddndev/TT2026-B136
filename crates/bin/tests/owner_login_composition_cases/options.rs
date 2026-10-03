use clap::{Args, Command, FromArgMatches};

use crate::serve_args::{OwnerLoginArgs, ServeArgs};

pub const OPTIONS: &[(&str, &str, &str)] = &[
    ("start-global-max", "START_GLOBAL_MAX", "17"),
    (
        "start-global-window-seconds",
        "START_GLOBAL_WINDOW_SECONDS",
        "61",
    ),
    ("start-owner-binding-max", "START_OWNER_BINDING_MAX", "3"),
    (
        "start-owner-binding-window-seconds",
        "START_OWNER_BINDING_WINDOW_SECONDS",
        "67",
    ),
    ("proof-global-max", "PROOF_GLOBAL_MAX", "23"),
    (
        "proof-global-window-seconds",
        "PROOF_GLOBAL_WINDOW_SECONDS",
        "71",
    ),
    ("proof-token-max", "PROOF_TOKEN_MAX", "5"),
    (
        "proof-token-window-seconds",
        "PROOF_TOKEN_WINDOW_SECONDS",
        "73",
    ),
    ("redis-connect-ms", "REDIS_CONNECT_MS", "83"),
    ("redis-io-ms", "REDIS_IO_MS", "89"),
];

pub fn command() -> Command {
    ServeArgs::augment_args(Command::new("serve"))
}

pub fn parse(extra: &[String]) -> Result<ServeArgs, clap::Error> {
    let mut arguments: Vec<String> = [
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
    .collect();
    arguments.extend_from_slice(extra);
    let matches = command()
        .mut_args(|arg| arg.env(None::<&str>))
        .try_get_matches_from(arguments)?;
    ServeArgs::from_arg_matches(&matches)
}

pub fn complete() -> OwnerLoginArgs {
    OwnerLoginArgs {
        enabled: true,
        start_global_max: Some(17),
        start_global_window_seconds: Some(61),
        start_owner_binding_max: Some(3),
        start_owner_binding_window_seconds: Some(67),
        proof_global_max: Some(23),
        proof_global_window_seconds: Some(71),
        proof_token_max: Some(5),
        proof_token_window_seconds: Some(73),
        redis_connect_ms: Some(83),
        redis_io_ms: Some(89),
    }
}

pub fn flags() -> Vec<String> {
    OPTIONS
        .iter()
        .flat_map(|(name, _, value)| [format!("--owner-login-{name}"), (*value).to_owned()])
        .collect()
}
