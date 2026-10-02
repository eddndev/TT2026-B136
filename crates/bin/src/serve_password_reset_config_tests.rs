use crate::{
    serve_email_credentials::EmailSettings,
    serve_password_reset_config::PasswordResetOptions,
    serve_password_reset_config_test_support::{invalid, options, resolve, PRIVATE_KEY},
};
use std::time::Duration;
use zeroize::Zeroizing;

#[test]
fn shared_key_alone_enables_neither_email_channel() {
    for key in [None, Some(Zeroizing::new(PRIVATE_KEY.into()))] {
        let settings = EmailSettings::resolve(key, None, None, PasswordResetOptions::default())
            .expect("disabled channels need no configuration");
        assert!(settings.alerts.is_none());
        assert!(settings.password_reset.is_none());
    }
}

#[test]
fn reset_enablement_does_not_depend_on_alert_configuration() {
    let settings = resolve(options()).expect("complete explicit reset configuration");
    assert!(settings.alerts.is_none());
    let reset = settings.password_reset.expect("reset enabled");
    assert_eq!(reset.policy.ttl_seconds(), 713);
    assert_eq!(reset.policy.max_pending_per_account(), 2);
    assert_eq!(reset.redis_connect_timeout, Duration::from_millis(83));
    assert_eq!(reset.redis_io_timeout, Duration::from_millis(89));
    assert_eq!(reset.email_connect_timeout, Duration::from_millis(97));
    assert_eq!(reset.email_total_timeout, Duration::from_millis(101));
}

#[test]
fn complete_alert_pair_can_be_enabled_without_reset_and_both_can_share_key() {
    for reset in [PasswordResetOptions::default(), options()] {
        let expected_reset = reset.enabled;
        let settings = EmailSettings::resolve(
            Some(Zeroizing::new(PRIVATE_KEY.into())),
            Some("alerts@example.test".into()),
            Some("https://qadra.example.test/login".into()),
            reset,
        )
        .expect("independent complete channels");
        let (alert, key) = settings.alerts.expect("alerts enabled").into_parts();
        assert_eq!(alert.from_email, "alerts@example.test");
        assert_eq!(alert.login_url, "https://qadra.example.test/login");
        assert!(key.as_str() == PRIVATE_KEY);
        assert_eq!(settings.password_reset.is_some(), expected_reset);
    }
}

#[test]
fn one_alert_setting_is_rejected_even_when_reset_uses_the_key() {
    for (sender, url) in [
        (Some("alerts@example.test".into()), None),
        (None, Some("https://qadra.example.test/login".into())),
    ] {
        invalid(EmailSettings::resolve(
            Some(Zeroizing::new(PRIVATE_KEY.into())),
            sender,
            url,
            options(),
        ));
    }
}

#[test]
fn disabled_reset_keeps_saved_options_inert_without_filling_alert_options() {
    let mut saved = options();
    saved.enabled = false;
    saved.public_url = Some("http://unapproved.invalid/private".into());
    saved.ttl_seconds = Some(0);
    let settings = EmailSettings::resolve(None, None, None, saved).expect("reset disabled");
    assert!(settings.alerts.is_none());
    assert!(settings.password_reset.is_none());
}

#[test]
fn enabled_channel_requires_private_key_without_revealing_it_in_errors() {
    for key in [
        None,
        Some(""),
        Some("key with spaces"),
        Some("key\r\nheader"),
    ] {
        invalid(EmailSettings::resolve(
            key.map(|value| Zeroizing::new(value.into())),
            None,
            None,
            options(),
        ));
    }
    invalid(EmailSettings::resolve(
        None,
        Some("alerts@example.test".into()),
        Some("https://qadra.example.test/login".into()),
        PasswordResetOptions::default(),
    ));
}

#[test]
fn enabled_reset_requires_every_explicit_option_without_operational_defaults() {
    let missing: &[fn(&mut PasswordResetOptions)] = &[
        |value| value.from_email = None,
        |value| value.public_url = None,
        |value| value.ttl_seconds = None,
        |value| value.max_pending = None,
        |value| value.request_global_max = None,
        |value| value.request_global_window_seconds = None,
        |value| value.request_email_max = None,
        |value| value.request_email_window_seconds = None,
        |value| value.complete_global_max = None,
        |value| value.complete_global_window_seconds = None,
        |value| value.complete_token_max = None,
        |value| value.complete_token_window_seconds = None,
        |value| value.redis_connect_ms = None,
        |value| value.redis_io_ms = None,
        |value| value.email_connect_ms = None,
        |value| value.email_total_ms = None,
    ];
    for remove in missing {
        let mut value = options();
        remove(&mut value);
        invalid(resolve(value));
    }
}

#[test]
fn invalid_policy_and_rate_boundaries_are_rejected_before_composition() {
    let invalid_options: &[fn(&mut PasswordResetOptions)] = &[
        |value| value.ttl_seconds = Some(0),
        |value| value.ttl_seconds = Some(u64::MAX),
        |value| value.max_pending = Some(0),
        |value| value.request_global_max = Some(0),
        |value| value.request_email_max = Some(0),
        |value| value.complete_global_max = Some(0),
        |value| value.complete_token_max = Some(0),
        |value| value.request_global_window_seconds = Some(0),
        |value| value.request_email_window_seconds = Some(86_401),
        |value| value.complete_global_window_seconds = Some(86_401),
        |value| value.complete_token_window_seconds = Some(u64::MAX),
    ];
    for change in invalid_options {
        let mut value = options();
        change(&mut value);
        invalid(resolve(value));
    }
}

#[test]
fn invalid_transport_options_fail_before_any_client_is_opened() {
    let invalid_options: &[fn(&mut PasswordResetOptions)] = &[
        |value| value.from_email = Some("Name <sender@example.test>".into()),
        |value| value.public_url = Some("http://qadra.example.test/".into()),
        |value| value.public_url = Some("https://qadra.example.test/?token=private".into()),
        |value| value.public_url = Some("https://qadra.example.test/#secret".into()),
        |value| value.redis_connect_ms = Some(0),
        |value| value.redis_io_ms = Some(0),
        |value| value.email_connect_ms = Some(0),
        |value| value.email_total_ms = Some(0),
        |value| value.email_connect_ms = Some(3_001),
        |value| value.email_total_ms = Some(10_001),
        |value| value.email_total_ms = Some(96),
    ];
    for change in invalid_options {
        let mut value = options();
        change(&mut value);
        invalid(resolve(value));
    }
}
