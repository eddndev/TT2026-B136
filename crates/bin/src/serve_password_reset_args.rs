//! Public recovery options; the delivery secret is never a command-line argument.

#[derive(Debug, Default, clap::Args)]
pub struct PasswordResetArgs {
    /// Enable recovery only with all explicit settings and RESEND_API_KEY.
    #[arg(
        long = "password-reset-enabled",
        env = "TT_PASSWORD_RESET_ENABLED",
        default_value_t = false
    )]
    pub enabled: bool,
    /// Bare sender address for recovery messages.
    #[arg(
        long = "password-reset-email-from",
        env = "TT_PASSWORD_RESET_EMAIL_FROM"
    )]
    pub from_email: Option<String>,
    /// Approved HTTPS origin root used by recovery links.
    #[arg(
        long = "password-reset-public-url",
        env = "TT_PASSWORD_RESET_PUBLIC_URL"
    )]
    pub public_url: Option<String>,
    /// Positive capability lifetime in seconds.
    #[arg(
        long = "password-reset-ttl-seconds",
        env = "TT_PASSWORD_RESET_TTL_SECONDS"
    )]
    pub ttl_seconds: Option<u64>,
    /// Positive maximum pending capabilities per account.
    #[arg(
        long = "password-reset-max-pending",
        env = "TT_PASSWORD_RESET_MAX_PENDING"
    )]
    pub max_pending: Option<u32>,
    /// Global request quota per configured window.
    #[arg(
        long = "password-reset-request-global-max",
        env = "TT_PASSWORD_RESET_REQUEST_GLOBAL_MAX"
    )]
    pub request_global_max: Option<u32>,
    /// Global request window in seconds, from 1 to 86400.
    #[arg(
        long = "password-reset-request-global-window-seconds",
        env = "TT_PASSWORD_RESET_REQUEST_GLOBAL_WINDOW_SECONDS"
    )]
    pub request_global_window_seconds: Option<u64>,
    /// Request quota per normalized email and configured window.
    #[arg(
        long = "password-reset-request-email-max",
        env = "TT_PASSWORD_RESET_REQUEST_EMAIL_MAX"
    )]
    pub request_email_max: Option<u32>,
    /// Per-email request window in seconds, from 1 to 86400.
    #[arg(
        long = "password-reset-request-email-window-seconds",
        env = "TT_PASSWORD_RESET_REQUEST_EMAIL_WINDOW_SECONDS"
    )]
    pub request_email_window_seconds: Option<u64>,
    /// Global completion quota per configured window.
    #[arg(
        long = "password-reset-complete-global-max",
        env = "TT_PASSWORD_RESET_COMPLETE_GLOBAL_MAX"
    )]
    pub complete_global_max: Option<u32>,
    /// Global completion window in seconds, from 1 to 86400.
    #[arg(
        long = "password-reset-complete-global-window-seconds",
        env = "TT_PASSWORD_RESET_COMPLETE_GLOBAL_WINDOW_SECONDS"
    )]
    pub complete_global_window_seconds: Option<u64>,
    /// Completion quota per capability digest and configured window.
    #[arg(
        long = "password-reset-complete-token-max",
        env = "TT_PASSWORD_RESET_COMPLETE_TOKEN_MAX"
    )]
    pub complete_token_max: Option<u32>,
    /// Per-digest completion window in seconds, from 1 to 86400.
    #[arg(
        long = "password-reset-complete-token-window-seconds",
        env = "TT_PASSWORD_RESET_COMPLETE_TOKEN_WINDOW_SECONDS"
    )]
    pub complete_token_window_seconds: Option<u64>,
    /// Positive Redis TCP connection timeout in milliseconds.
    #[arg(
        long = "password-reset-redis-connect-ms",
        env = "TT_PASSWORD_RESET_REDIS_CONNECT_MS"
    )]
    pub redis_connect_ms: Option<u64>,
    /// Positive established Redis socket timeout in milliseconds.
    #[arg(
        long = "password-reset-redis-io-ms",
        env = "TT_PASSWORD_RESET_REDIS_IO_MS"
    )]
    pub redis_io_ms: Option<u64>,
    /// Email connection timeout in milliseconds, at most 3000.
    #[arg(
        long = "password-reset-email-connect-ms",
        env = "TT_PASSWORD_RESET_EMAIL_CONNECT_MS"
    )]
    pub email_connect_ms: Option<u64>,
    /// Email total timeout in milliseconds, at most 10000 and at least connect.
    #[arg(
        long = "password-reset-email-total-ms",
        env = "TT_PASSWORD_RESET_EMAIL_TOTAL_MS"
    )]
    pub email_total_ms: Option<u64>,
}
