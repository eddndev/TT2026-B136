//! PostgreSQL, Redis, and encryption adapters for identity workflows.

mod postgres;
mod redis;
mod secret;

pub use postgres::PostgresUserRepository;
pub use redis::RedisSessionStore;
pub use secret::AesGcmSecretProtector;
