use crate::{
    serve_email_credentials::EmailSettings, serve_password_reset_config::PasswordResetOptions,
};
use application::ApplicationError;
use zeroize::Zeroizing;

pub const PRIVATE_KEY: &str = "private-reset-configuration-key";

pub fn options() -> PasswordResetOptions {
    PasswordResetOptions {
        enabled: true,
        from_email: Some("recovery@example.test".into()),
        public_url: Some("https://qadra.example.test/".into()),
        ttl_seconds: Some(713),
        max_pending: Some(2),
        request_global_max: Some(13),
        request_global_window_seconds: Some(61),
        request_email_max: Some(3),
        request_email_window_seconds: Some(67),
        complete_global_max: Some(17),
        complete_global_window_seconds: Some(71),
        complete_token_max: Some(5),
        complete_token_window_seconds: Some(73),
        redis_connect_ms: Some(83),
        redis_io_ms: Some(89),
        email_connect_ms: Some(97),
        email_total_ms: Some(101),
    }
}

pub fn resolve(options: PasswordResetOptions) -> Result<EmailSettings, ApplicationError> {
    EmailSettings::resolve(
        Some(Zeroizing::new(PRIVATE_KEY.into())),
        None,
        None,
        options,
    )
}

pub fn invalid(result: Result<EmailSettings, ApplicationError>) {
    let error = match result {
        Ok(_) => panic!("invalid email configuration was accepted"),
        Err(error) => error,
    };
    assert!(matches!(error, ApplicationError::InvalidConfiguration(_)));
    assert!(!error.to_string().contains(PRIVATE_KEY));
}
