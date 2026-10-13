//! Fixed CA credentials, session namespace and admission; no office workflows.
use std::sync::Arc;

use application::{
    ca_access::CaOperatorAccess,
    identity::{IdentityPorts, IdentityService, SessionPolicy, UserRepository},
};
use domain::identity::Role;
use infrastructure::{
    with_validated_ca_identity_postgres, AesGcmSecretProtector, Argon2idHasher, PostgresAuditLog,
    PostgresUserRepository, RandomRecoveryCodeGenerator, RedisSessionStore, SystemClock,
    TotpRsProvider,
};
use zeroize::Zeroizing;

use super::config::Settings;

pub fn open(settings: &Settings) -> anyhow::Result<Arc<CaOperatorAccess>> {
    let operator = settings.operator()?;
    let (users, identity) = open_identity(
        &settings.database_url,
        &settings.database_name,
        &settings.database_role,
        &settings.redis_url,
        settings.kek()?,
        settings.policy()?,
    )?;
    let user = users
        .find_by_id(operator.id)?
        .ok_or_else(|| anyhow::anyhow!("the configured CA operator is not enrolled"))?;
    if user.email != operator.email || !user.active || user.role != Role::Client {
        anyhow::bail!("the configured CA operator is not available");
    }
    Ok(Arc::new(CaOperatorAccess::new(
        Arc::new(identity),
        operator,
    )))
}

/// Opens only CA runtime adapters, including when no initial operator exists.
pub fn open_identity(
    database_url: &str,
    database_name: &str,
    database_role: &str,
    redis_url: &str,
    kek: Zeroizing<Vec<u8>>,
    policy: SessionPolicy,
) -> anyhow::Result<(Arc<PostgresUserRepository>, IdentityService)> {
    let (users, audit) = with_validated_ca_identity_postgres(
        database_url,
        database_name,
        database_role,
        |source| -> Result<_, application::ApplicationError> {
            Ok((
                PostgresUserRepository::open(source)?,
                PostgresAuditLog::open(source)?,
            ))
        },
    )?;
    let users = Arc::new(users);
    let identity = IdentityService::with_session_policy(
        IdentityPorts {
            users: users.clone(),
            sessions: Arc::new(RedisSessionStore::connect_ca(redis_url)?),
            passwords: Arc::new(Argon2idHasher::new()),
            totp: Arc::new(TotpRsProvider::new()),
            recovery: Arc::new(RandomRecoveryCodeGenerator),
            secrets: Arc::new(AesGcmSecretProtector::new(kek)?),
            clock: Arc::new(SystemClock::new()),
            audit_log: Box::new(audit),
        },
        policy,
    );
    Ok((users, identity))
}
