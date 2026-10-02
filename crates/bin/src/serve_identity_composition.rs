//! Identity adapters composed with the server-selected session policy.

use anyhow::Context;
use application::identity::{IdentityPorts, IdentityService, IdentityWorkflow, SessionPolicy};
use infrastructure::{
    AesGcmSecretProtector, Argon2idHasher, PostgresAuditLog, PostgresConnectionSource,
    PostgresUserRepository, RandomRecoveryCodeGenerator, RedisSessionStore, SystemClock,
    TotpRsProvider,
};
use std::sync::Arc;
use zeroize::Zeroizing;

pub(crate) fn open(
    database: &(impl PostgresConnectionSource + ?Sized),
    redis_url: &str,
    kek: Zeroizing<Vec<u8>>,
    policy: SessionPolicy,
) -> anyhow::Result<Arc<dyn IdentityWorkflow>> {
    let ports = IdentityPorts {
        users: Arc::new(
            PostgresUserRepository::open(database)
                .context("cannot initialize PostgreSQL user repository")?,
        ),
        sessions: Arc::new(
            RedisSessionStore::connect(redis_url)
                .context("cannot initialize Redis session store")?,
        ),
        passwords: Arc::new(Argon2idHasher::new()),
        totp: Arc::new(TotpRsProvider::new()),
        recovery: Arc::new(RandomRecoveryCodeGenerator),
        secrets: Arc::new(
            AesGcmSecretProtector::new(kek).context("cannot initialize TOTP secret protection")?,
        ),
        clock: Arc::new(SystemClock::new()),
        audit_log: Box::new(
            PostgresAuditLog::open(database).context("cannot open PostgreSQL audit log")?,
        ),
    };
    Ok(Arc::new(IdentityService::with_session_policy(
        ports, policy,
    )))
}
