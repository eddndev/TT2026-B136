use application::ApplicationError;

/// One positive fixed-window budget, bounded to one day of retained state.
#[derive(Clone, Copy)]
pub struct PasswordResetRateLimit {
    pub(super) maximum: u32,
    pub(super) window_ms: u64,
}

impl PasswordResetRateLimit {
    /// This technical ceiling is not an operational default.
    pub fn new(max_attempts: u32, window_seconds: u64) -> Result<Self, ApplicationError> {
        if max_attempts == 0 || !(1..=86_400).contains(&window_seconds) {
            return Err(ApplicationError::InvalidConfiguration(
                "password reset budgets require a positive quota and a 1..=86400 second window"
                    .into(),
            ));
        }
        Ok(Self {
            maximum: max_attempts,
            window_ms: window_seconds * 1_000,
        })
    }
}

/// Explicit independent global and subject budgets for each reset operation.
#[derive(Clone, Copy)]
pub struct PasswordResetRatePolicy {
    pub(super) request_global: PasswordResetRateLimit,
    pub(super) request_email: PasswordResetRateLimit,
    pub(super) completion_global: PasswordResetRateLimit,
    pub(super) completion_digest: PasswordResetRateLimit,
}

impl PasswordResetRatePolicy {
    pub fn new(
        request_global: PasswordResetRateLimit,
        request_email: PasswordResetRateLimit,
        completion_global: PasswordResetRateLimit,
        completion_digest: PasswordResetRateLimit,
    ) -> Self {
        Self {
            request_global,
            request_email,
            completion_global,
            completion_digest,
        }
    }
}
