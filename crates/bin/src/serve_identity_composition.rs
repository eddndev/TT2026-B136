//! Identity adapters composed with the server-selected session policy.

use anyhow::Context;
use application::identity::{
    certificate_login::{CertificateLoginPorts, OwnerLoginWorkflow},
    IdentityPorts, IdentityService, IdentityWorkflow, SessionPolicy,
};
use infrastructure::{
    certificates::InternalRsaOwnerLoginVerifier, identity::RedisOwnerLoginRuntime,
    AesGcmSecretProtector, Argon2idHasher, PostgresAuditLog, PostgresConnectionSource,
    PostgresOwnerCertificateStore, PostgresUserRepository, RandomRecoveryCodeGenerator,
    RedisSessionStore, RingSha256Hasher, SystemClock, TotpRsProvider,
};
use std::sync::Arc;
use zeroize::Zeroizing;

use crate::serve_owner_login_config::OwnerLoginSettings;

/// Both inbound views share the same identity, session policy and provenance guard.
pub(crate) struct IdentityComponents {
    pub identity: Arc<dyn IdentityWorkflow>,
    pub certificate_login: Option<Arc<dyn OwnerLoginWorkflow>>,
}

pub(crate) fn open(
    database: &(impl PostgresConnectionSource + ?Sized),
    redis_url: &str,
    kek: Zeroizing<Vec<u8>>,
    policy: SessionPolicy,
    certificate_settings: Option<&OwnerLoginSettings>,
) -> anyhow::Result<IdentityComponents> {
    let clock = Arc::new(SystemClock::new());
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
        clock: clock.clone(),
        audit_log: Box::new(
            PostgresAuditLog::open(database).context("cannot open PostgreSQL audit log")?,
        ),
    };
    let certificate = certificate_settings
        .map(|settings| -> anyhow::Result<_> {
            Ok(CertificateLoginPorts {
                authority: Arc::new(
                    PostgresOwnerCertificateStore::open(database, clock)
                        .context("cannot open PostgreSQL certificate login authority")?,
                ),
                runtime: Arc::new(
                    RedisOwnerLoginRuntime::connect(
                        redis_url,
                        settings.rates,
                        settings.redis_connect_timeout,
                        settings.redis_io_timeout,
                    )
                    .context("cannot configure certificate login runtime")?,
                ),
                verifier: Arc::new(InternalRsaOwnerLoginVerifier::new()),
                hasher: Arc::new(RingSha256Hasher::new()),
            })
        })
        .transpose()?;
    Ok(compose(ports, policy, certificate))
}

pub(crate) fn compose(
    ports: IdentityPorts,
    policy: SessionPolicy,
    certificate: Option<CertificateLoginPorts>,
) -> IdentityComponents {
    match certificate {
        Some(certificate) => {
            let identity = Arc::new(IdentityService::with_certificate_login(
                ports,
                policy,
                certificate,
            ));
            IdentityComponents {
                identity: identity.clone(),
                certificate_login: Some(identity),
            }
        }
        None => IdentityComponents {
            identity: Arc::new(IdentityService::with_session_policy(ports, policy)),
            certificate_login: None,
        },
    }
}
