//! Application settings loaded from the environment.
//!
//! Only non-secret settings live here. Secret material (key material, service
//! credentials) is loaded next to the code that uses it, so it can be held in
//! zeroizing buffers and kept out of general configuration and logs.

use std::env;

/// Runtime settings read from environment variables (and an optional `.env`).
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Log filter directive, for example `info` or `despacho_cli=debug`.
    pub rust_log: String,
    /// Base URL of the timestamp authority, when configured.
    pub cincel_base_url: Option<String>,
    /// Database connection string, when configured.
    pub database_url: Option<String>,
    /// Redis connection string, when configured.
    pub redis_url: Option<String>,
}

impl AppConfig {
    /// Reads settings from the environment, applying defaults for what is
    /// missing. Never fails: a missing optional value stays `None`.
    pub fn load() -> Self {
        Self {
            rust_log: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            cincel_base_url: env::var("CINCEL_BASE_URL").ok(),
            database_url: env::var("DATABASE_URL").ok(),
            redis_url: env::var("REDIS_URL").ok(),
        }
    }
}
