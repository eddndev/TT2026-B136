//! Pure validation of explicitly enabled Owner certificate login settings.

use std::time::{Duration, Instant};

use application::ApplicationError;
use infrastructure::identity::{OwnerLoginRateLimit, OwnerLoginRatePolicy};

use crate::serve_args::OwnerLoginArgs;

pub(crate) struct OwnerLoginSettings {
    pub rates: OwnerLoginRatePolicy,
    pub redis_connect_timeout: Duration,
    pub redis_io_timeout: Duration,
}

impl OwnerLoginSettings {
    /// Saved numeric options are inert until the channel is explicitly enabled.
    pub(crate) fn resolve(options: &OwnerLoginArgs) -> Result<Option<Self>, ApplicationError> {
        if !options.enabled {
            return Ok(None);
        }
        let rates = OwnerLoginRatePolicy::new(
            rate(
                options.start_global_max,
                options.start_global_window_seconds,
            )?,
            rate(
                options.start_owner_binding_max,
                options.start_owner_binding_window_seconds,
            )?,
            rate(
                options.proof_global_max,
                options.proof_global_window_seconds,
            )?,
            rate(options.proof_token_max, options.proof_token_window_seconds)?,
        );
        Ok(Some(Self {
            rates,
            redis_connect_timeout: timeout(options.redis_connect_ms)?,
            redis_io_timeout: timeout(options.redis_io_ms)?,
        }))
    }
}

fn required<T>(value: Option<T>) -> Result<T, ApplicationError> {
    value.ok_or_else(invalid)
}

fn rate(
    maximum: Option<u32>,
    window: Option<u64>,
) -> Result<OwnerLoginRateLimit, ApplicationError> {
    OwnerLoginRateLimit::new(required(maximum)?, required(window)?).map_err(|_| invalid())
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
        "invalid or incomplete owner certificate login settings".into(),
    )
}
