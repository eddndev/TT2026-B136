use std::{
    ffi::OsStr,
    time::{Duration, Instant},
};

use super::options::{command, complete, flags, parse, OPTIONS};
use crate::{serve_args::OwnerLoginArgs, serve_owner_login_config::OwnerLoginSettings};
use application::ApplicationError;

type Maximum = fn(&mut OwnerLoginArgs) -> &mut Option<u32>;
type DurationOption = fn(&mut OwnerLoginArgs) -> &mut Option<u64>;
const MAXIMA: [Maximum; 4] = [
    |o| &mut o.start_global_max,
    |o| &mut o.start_owner_binding_max,
    |o| &mut o.proof_global_max,
    |o| &mut o.proof_token_max,
];
const WINDOWS: [DurationOption; 4] = [
    |o| &mut o.start_global_window_seconds,
    |o| &mut o.start_owner_binding_window_seconds,
    |o| &mut o.proof_global_window_seconds,
    |o| &mut o.proof_token_window_seconds,
];
const TIMEOUTS: [DurationOption; 2] = [|o| &mut o.redis_connect_ms, |o| &mut o.redis_io_ms];

fn invalid(options: &OwnerLoginArgs) {
    assert!(matches!(
        OwnerLoginSettings::resolve(options),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}

#[test]
fn options_are_explicit_and_saved_values_do_not_enable_the_channel() {
    let command = command();
    let enabled = command
        .get_arguments()
        .find(|arg| arg.get_long() == Some("owner-login-enabled"))
        .unwrap();
    assert_eq!(
        enabled.get_env(),
        Some(OsStr::new("TT_OWNER_LOGIN_ENABLED"))
    );
    for (name, suffix, _) in OPTIONS {
        let name = format!("owner-login-{name}");
        let argument = command
            .get_arguments()
            .find(|arg| arg.get_long() == Some(name.as_str()))
            .unwrap();
        let env = format!("TT_OWNER_LOGIN_{suffix}");
        assert_eq!(argument.get_env(), Some(OsStr::new(&env)));
        assert!(argument.get_default_values().is_empty(), "{name}");
        assert!(!argument.is_required_set(), "{name}");
    }
    let absent = parse(&[]).unwrap().owner_login;
    assert!(!absent.enabled);
    assert!(OwnerLoginSettings::resolve(&absent).unwrap().is_none());
    let saved = parse(&flags()).unwrap().owner_login;
    assert!(!saved.enabled);
    assert!(OwnerLoginSettings::resolve(&saved).unwrap().is_none());
    let mut inactive = complete();
    inactive.enabled = false;
    inactive.start_global_max = Some(0);
    inactive.proof_token_window_seconds = Some(u64::MAX);
    inactive.redis_io_ms = None;
    assert!(OwnerLoginSettings::resolve(&inactive).unwrap().is_none());
}

#[test]
fn enabled_resolution_requires_each_quota_window_and_timeout_without_clamping() {
    for maximum in MAXIMA {
        for value in [None, Some(0)] {
            let mut options = complete();
            *maximum(&mut options) = value;
            invalid(&options);
        }
        for value in [1, u32::MAX] {
            let mut options = complete();
            *maximum(&mut options) = Some(value);
            assert!(OwnerLoginSettings::resolve(&options).unwrap().is_some());
        }
    }
    for window in WINDOWS {
        for value in [None, Some(0), Some(86_401), Some(u64::MAX)] {
            let mut options = complete();
            *window(&mut options) = value;
            invalid(&options);
        }
        for value in [1, 86_400] {
            let mut options = complete();
            *window(&mut options) = Some(value);
            assert!(OwnerLoginSettings::resolve(&options).unwrap().is_some());
        }
    }
    for timeout in TIMEOUTS {
        for value in [None, Some(0)] {
            let mut options = complete();
            *timeout(&mut options) = value;
            invalid(&options);
        }
    }
    let settings = OwnerLoginSettings::resolve(&complete()).unwrap().unwrap();
    assert_eq!(settings.redis_connect_timeout, Duration::from_millis(83));
    assert_eq!(settings.redis_io_timeout, Duration::from_millis(89));
    // Milliseconds up to u64::MAX can be representable on some platforms.
    for timeout in TIMEOUTS {
        let mut options = complete();
        *timeout(&mut options) = Some(u64::MAX);
        let representable = Instant::now()
            .checked_add(Duration::from_millis(u64::MAX))
            .is_some();
        assert_eq!(OwnerLoginSettings::resolve(&options).is_ok(), representable);
    }
}

#[test]
fn command_line_preserves_complete_settings_and_rejects_invalid_numeric_syntax() {
    let mut arguments = flags();
    arguments.push("--owner-login-enabled".into());
    let parsed = parse(&arguments).unwrap().owner_login;
    assert!(parsed.enabled);
    assert_eq!(
        (parsed.start_global_max, parsed.start_owner_binding_max),
        (Some(17), Some(3))
    );
    assert_eq!(
        (parsed.proof_global_max, parsed.proof_token_max),
        (Some(23), Some(5))
    );
    assert_eq!(parsed.start_global_window_seconds, Some(61));
    assert_eq!(parsed.start_owner_binding_window_seconds, Some(67));
    assert_eq!(parsed.proof_global_window_seconds, Some(71));
    assert_eq!(parsed.proof_token_window_seconds, Some(73));
    assert!(OwnerLoginSettings::resolve(&parsed).unwrap().is_some());
    for (name, _, _) in OPTIONS {
        for value in ["-1", "1.5", "invalid", "18446744073709551616"] {
            assert!(
                parse(&[format!("--owner-login-{name}={value}")]).is_err(),
                "{name}"
            );
        }
        if name.ends_with("-max") {
            assert!(
                parse(&[format!("--owner-login-{name}=4294967296")]).is_err(),
                "{name}"
            );
        }
    }
}
