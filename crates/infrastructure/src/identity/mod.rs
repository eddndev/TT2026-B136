//! PostgreSQL, Redis, and encryption adapters for identity workflows.

mod invitation_secret;
mod owner_login_runtime;
mod password_reset;
mod password_reset_restore;
mod password_reset_runtime;
pub(crate) mod postgres;
mod redis;
mod secret;

#[cfg(test)]
mod invitation_secret_tests;

pub use invitation_secret::AesGcmInvitationProtector;
pub use owner_login_runtime::{OwnerLoginRateLimit, OwnerLoginRatePolicy, RedisOwnerLoginRuntime};
pub use password_reset::PostgresPasswordResetRepository;
pub use password_reset_restore::{
    invalidate_restored_invitations, invalidate_restored_password_resets, PasswordResetRestoreHead,
    PasswordResetRestoreRequest, PasswordResetRestoreResult,
};
pub use password_reset_runtime::{
    PasswordResetRateLimit, PasswordResetRatePolicy, RandomResetTokenSource,
    RedisPasswordResetLimiter,
};
pub use postgres::PostgresUserRepository;
pub use redis::RedisSessionStore;
pub use secret::AesGcmSecretProtector;

mod invitation_limiter;
pub use invitation_limiter::RedisInvitationLimiter;
