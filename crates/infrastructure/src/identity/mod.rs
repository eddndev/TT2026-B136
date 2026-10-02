//! PostgreSQL, Redis, and encryption adapters for identity workflows.

mod password_reset;
mod password_reset_restore;
mod postgres;
mod redis;
mod secret;

pub use password_reset::PostgresPasswordResetRepository;
pub use password_reset_restore::{
    invalidate_restored_password_resets, PasswordResetRestoreHead, PasswordResetRestoreRequest,
    PasswordResetRestoreResult,
};
pub use postgres::PostgresUserRepository;
pub use redis::RedisSessionStore;
pub use secret::AesGcmSecretProtector;
