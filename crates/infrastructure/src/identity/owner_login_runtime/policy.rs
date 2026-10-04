use application::ApplicationError;

/// A positive fixed-window quota with a technical retention ceiling of one day.
#[derive(Clone, Copy)]
pub struct OwnerLoginRateLimit {
    pub(super) maximum: u32,
    pub(super) window_ms: u64,
}

impl OwnerLoginRateLimit {
    pub fn new(max_attempts: u32, window_seconds: u64) -> Result<Self, ApplicationError> {
        if max_attempts == 0 || !(1..=86_400).contains(&window_seconds) {
            return Err(ApplicationError::InvalidConfiguration(
                "certificate login budgets require a positive quota and a 1..=86400 second window"
                    .into(),
            ));
        }
        Ok(Self {
            maximum: max_attempts,
            window_ms: window_seconds * 1000,
        })
    }
}

/// Explicit independent budgets for challenge preparation and proof submission.
#[derive(Clone, Copy)]
pub struct OwnerLoginRatePolicy {
    pub(super) start_global: OwnerLoginRateLimit,
    pub(super) start_owner_binding: OwnerLoginRateLimit,
    pub(super) proof_global: OwnerLoginRateLimit,
    pub(super) proof_token: OwnerLoginRateLimit,
}

impl OwnerLoginRatePolicy {
    pub fn new(
        start_global: OwnerLoginRateLimit,
        start_owner_binding: OwnerLoginRateLimit,
        proof_global: OwnerLoginRateLimit,
        proof_token: OwnerLoginRateLimit,
    ) -> Self {
        Self {
            start_global,
            start_owner_binding,
            proof_global,
            proof_token,
        }
    }
}
