//! PostgreSQL, Redis, and encryption adapters for identity workflows.

mod owner_login_runtime;
mod password_reset;
mod password_reset_restore;
mod password_reset_runtime;
mod postgres;
mod redis;
mod secret;

pub use owner_login_runtime::{OwnerLoginRateLimit, OwnerLoginRatePolicy, RedisOwnerLoginRuntime};
pub use password_reset::PostgresPasswordResetRepository;
pub use password_reset_restore::{
    invalidate_restored_password_resets, PasswordResetRestoreHead, PasswordResetRestoreRequest,
    PasswordResetRestoreResult,
};
pub use password_reset_runtime::{
    PasswordResetRateLimit, PasswordResetRatePolicy, RandomResetTokenSource,
    RedisPasswordResetLimiter,
};
pub use postgres::PostgresUserRepository;
pub use redis::RedisSessionStore;
pub use secret::AesGcmSecretProtector;
