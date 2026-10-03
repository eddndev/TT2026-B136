//! Explicit opt-in and budgets for the public Owner certificate first factor.

#[derive(Debug, Default, clap::Args)]
pub struct OwnerLoginArgs {
    /// Enable certificate login only with all explicit budgets and timeouts.
    #[arg(
        id = "owner_login_enabled",
        long = "owner-login-enabled",
        env = "TT_OWNER_LOGIN_ENABLED",
        default_value_t = false
    )]
    pub enabled: bool,
    /// Positive global challenge quota per configured window.
    #[arg(
        id = "owner_login_start_global_max",
        long = "owner-login-start-global-max",
        env = "TT_OWNER_LOGIN_START_GLOBAL_MAX"
    )]
    pub start_global_max: Option<u32>,
    /// Global challenge window in seconds, from 1 to 86400.
    #[arg(
        id = "owner_login_start_global_window_seconds",
        long = "owner-login-start-global-window-seconds",
        env = "TT_OWNER_LOGIN_START_GLOBAL_WINDOW_SECONDS"
    )]
    pub start_global_window_seconds: Option<u64>,
    /// Positive challenge quota for each selected Owner and binding.
    #[arg(
        id = "owner_login_start_owner_binding_max",
        long = "owner-login-start-owner-binding-max",
        env = "TT_OWNER_LOGIN_START_OWNER_BINDING_MAX"
    )]
    pub start_owner_binding_max: Option<u32>,
    /// Owner and binding challenge window in seconds, from 1 to 86400.
    #[arg(
        id = "owner_login_start_owner_binding_window_seconds",
        long = "owner-login-start-owner-binding-window-seconds",
        env = "TT_OWNER_LOGIN_START_OWNER_BINDING_WINDOW_SECONDS"
    )]
    pub start_owner_binding_window_seconds: Option<u64>,
    /// Positive global proof quota per configured window.
    #[arg(
        id = "owner_login_proof_global_max",
        long = "owner-login-proof-global-max",
        env = "TT_OWNER_LOGIN_PROOF_GLOBAL_MAX"
    )]
    pub proof_global_max: Option<u32>,
    /// Global proof window in seconds, from 1 to 86400.
    #[arg(
        id = "owner_login_proof_global_window_seconds",
        long = "owner-login-proof-global-window-seconds",
        env = "TT_OWNER_LOGIN_PROOF_GLOBAL_WINDOW_SECONDS"
    )]
    pub proof_global_window_seconds: Option<u64>,
    /// Positive proof quota per challenge token fingerprint.
    #[arg(
        id = "owner_login_proof_token_max",
        long = "owner-login-proof-token-max",
        env = "TT_OWNER_LOGIN_PROOF_TOKEN_MAX"
    )]
    pub proof_token_max: Option<u32>,
    /// Per-token proof window in seconds, from 1 to 86400.
    #[arg(
        id = "owner_login_proof_token_window_seconds",
        long = "owner-login-proof-token-window-seconds",
        env = "TT_OWNER_LOGIN_PROOF_TOKEN_WINDOW_SECONDS"
    )]
    pub proof_token_window_seconds: Option<u64>,
    /// Positive Redis TCP connection timeout in milliseconds.
    #[arg(
        id = "owner_login_redis_connect_ms",
        long = "owner-login-redis-connect-ms",
        env = "TT_OWNER_LOGIN_REDIS_CONNECT_MS"
    )]
    pub redis_connect_ms: Option<u64>,
    /// Positive established Redis socket timeout in milliseconds.
    #[arg(
        id = "owner_login_redis_io_ms",
        long = "owner-login-redis-io-ms",
        env = "TT_OWNER_LOGIN_REDIS_IO_MS"
    )]
    pub redis_io_ms: Option<u64>,
}
