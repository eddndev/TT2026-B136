//! Independent email channel enablement with one explicitly supplied secret.

use crate::{
    serve_alert_config::AlertEmailSettings,
    serve_password_reset_config::{PasswordResetOptions, PasswordResetSettings},
};
use anyhow::Context;
use application::ApplicationError;
use zeroize::Zeroizing;

pub(crate) fn load(args: &crate::serve_args::ServeArgs) -> anyhow::Result<EmailSettings> {
    let enabled = args.password_reset.enabled
        || args.alert_email_from.is_some()
        || args.alert_login_url.is_some();
    let key = if enabled {
        match std::env::var("RESEND_API_KEY") {
            Ok(value) => Some(Zeroizing::new(value)),
            Err(std::env::VarError::NotPresent) => None,
            Err(std::env::VarError::NotUnicode(_)) => {
                anyhow::bail!("RESEND_API_KEY must be valid text")
            }
        }
    } else {
        None
    };
    EmailSettings::resolve(
        key,
        args.alert_email_from.clone(),
        args.alert_login_url.clone(),
        options(&args.password_reset),
    )
    .context("cannot configure recovery and alert email")
}

fn options(args: &crate::serve_args::PasswordResetArgs) -> PasswordResetOptions {
    PasswordResetOptions {
        enabled: args.enabled,
        from_email: args.from_email.clone(),
        public_url: args.public_url.clone(),
        ttl_seconds: args.ttl_seconds,
        max_pending: args.max_pending,
        request_global_max: args.request_global_max,
        request_global_window_seconds: args.request_global_window_seconds,
        request_email_max: args.request_email_max,
        request_email_window_seconds: args.request_email_window_seconds,
        complete_global_max: args.complete_global_max,
        complete_global_window_seconds: args.complete_global_window_seconds,
        complete_token_max: args.complete_token_max,
        complete_token_window_seconds: args.complete_token_window_seconds,
        redis_connect_ms: args.redis_connect_ms,
        redis_io_ms: args.redis_io_ms,
        email_connect_ms: args.email_connect_ms,
        email_total_ms: args.email_total_ms,
    }
}

pub(crate) struct EmailSettings {
    pub alerts: Option<AlertEmailSettings>,
    pub password_reset: Option<PasswordResetSettings>,
}

impl EmailSettings {
    /// Does not read environment variables or initialize network clients.
    pub(crate) fn resolve(
        api_key: Option<Zeroizing<String>>,
        alert_from: Option<String>,
        alert_login: Option<String>,
        reset: PasswordResetOptions,
    ) -> Result<Self, ApplicationError> {
        let alerts = if alert_from.is_none() && alert_login.is_none() {
            None
        } else {
            AlertEmailSettings::from_optional(
                api_key.as_ref().map(|key| key.as_str().to_owned()),
                alert_from,
                alert_login,
            )?
        };
        let password_reset = PasswordResetSettings::resolve(reset, api_key)?;
        Ok(Self {
            alerts,
            password_reset,
        })
    }
}
