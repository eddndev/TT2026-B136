//! Pure validation of explicitly enabled password recovery settings.

use application::{identity::password_reset::ResetPolicy, ApplicationError};
use infrastructure::{
    identity::{PasswordResetRateLimit, PasswordResetRatePolicy},
    password_reset_email::PasswordResetEmailConfiguration,
};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

/// Saved values do not enable recovery; every enabled setting is explicit.
#[derive(Default)]
pub(crate) struct PasswordResetOptions {
    pub enabled: bool,
    pub from_email: Option<String>,
    pub public_url: Option<String>,
    pub ttl_seconds: Option<u64>,
    pub max_pending: Option<u32>,
    pub request_global_max: Option<u32>,
    pub request_global_window_seconds: Option<u64>,
    pub request_email_max: Option<u32>,
    pub request_email_window_seconds: Option<u64>,
    pub complete_global_max: Option<u32>,
    pub complete_global_window_seconds: Option<u64>,
    pub complete_token_max: Option<u32>,
    pub complete_token_window_seconds: Option<u64>,
    pub redis_connect_ms: Option<u64>,
    pub redis_io_ms: Option<u64>,
    pub email_connect_ms: Option<u64>,
    pub email_total_ms: Option<u64>,
}

/// Contains a delivery secret and deliberately has no Debug implementation.
pub(crate) struct PasswordResetSettings {
    pub configuration: PasswordResetEmailConfiguration,
    pub api_key: Zeroizing<String>,
    pub policy: ResetPolicy,
    pub rates: PasswordResetRatePolicy,
    pub redis_connect_timeout: Duration,
    pub redis_io_timeout: Duration,
    pub email_connect_timeout: Duration,
    pub email_total_timeout: Duration,
}

impl PasswordResetSettings {
    pub(crate) fn resolve(
        options: PasswordResetOptions,
        api_key: Option<Zeroizing<String>>,
    ) -> Result<Option<Self>, ApplicationError> {
        if !options.enabled {
            return Ok(None);
        }
        let api_key = api_key.ok_or_else(invalid)?;
        if api_key.is_empty()
            || api_key.len() > 1024
            || !api_key.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(invalid());
        }
        let configuration = PasswordResetEmailConfiguration::new(
            required(options.from_email)?,
            required(options.public_url)?,
        )?;
        let policy = ResetPolicy::new(
            required(options.ttl_seconds)?,
            required(options.max_pending)?,
        )?;
        let rates = PasswordResetRatePolicy::new(
            rate(
                options.request_global_max,
                options.request_global_window_seconds,
            )?,
            rate(
                options.request_email_max,
                options.request_email_window_seconds,
            )?,
            rate(
                options.complete_global_max,
                options.complete_global_window_seconds,
            )?,
            rate(
                options.complete_token_max,
                options.complete_token_window_seconds,
            )?,
        );
        let redis_connect_timeout = timeout(options.redis_connect_ms)?;
        let redis_io_timeout = timeout(options.redis_io_ms)?;
        let email_connect_timeout = timeout(options.email_connect_ms)?;
        let email_total_timeout = timeout(options.email_total_ms)?;
        if email_connect_timeout > Duration::from_secs(3)
            || email_total_timeout > Duration::from_secs(10)
            || email_connect_timeout > email_total_timeout
        {
            return Err(invalid());
        }
        Ok(Some(Self {
            configuration,
            api_key,
            policy,
            rates,
            redis_connect_timeout,
            redis_io_timeout,
            email_connect_timeout,
            email_total_timeout,
        }))
    }
}

fn required<T>(value: Option<T>) -> Result<T, ApplicationError> {
    value.ok_or_else(invalid)
}

fn rate(
    maximum: Option<u32>,
    window: Option<u64>,
) -> Result<PasswordResetRateLimit, ApplicationError> {
    PasswordResetRateLimit::new(required(maximum)?, required(window)?)
}

fn timeout(milliseconds: Option<u64>) -> Result<Duration, ApplicationError> {
    let duration = Duration::from_millis(required(milliseconds)?);
    if duration.is_zero() || Instant::now().checked_add(duration).is_none() {
        return Err(invalid());
    }
    Ok(duration)
}

fn invalid() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "invalid or incomplete password recovery settings".into(),
    )
}
