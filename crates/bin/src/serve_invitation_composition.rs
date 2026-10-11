//! Construct independently activated invitations on the existing identity boundary.
use crate::{
    serve_invitation_config::InvitationSettings, serve_invitation_runtime::InvitationConsumer,
};
use anyhow::Context;
use application::identity::{invitations::*, IdentityWorkflow};
use infrastructure::{
    identity::{RandomResetTokenSource, RedisInvitationLimiter},
    invitation_email::ResendInvitationDelivery,
    AesGcmInvitationProtector, AesGcmSecretProtector, Argon2idHasher, PostgresConnectionSource,
    PostgresInvitationStore, PostgresUserRepository, RandomRecoveryCodeGenerator, RingSha256Hasher,
    SystemClock, TotpRsProvider,
};
use std::{sync::Arc, time::Duration};
use zeroize::Zeroizing;

pub(crate) fn open(
    database: &(impl PostgresConnectionSource + ?Sized),
    redis_url: &str,
    identity: Arc<dyn IdentityWorkflow>,
    kek: Zeroizing<Vec<u8>>,
    settings: Option<InvitationSettings>,
) -> anyhow::Result<(Option<web::InvitationHttp>, Option<InvitationConsumer>)> {
    let Some(settings) = settings else {
        return Ok((None, None));
    };
    let delivery = Arc::new(ResendInvitationDelivery::new(
        settings.api_key,
        settings.configuration,
        Duration::from_secs(2),
        Duration::from_secs(5),
    )?);
    compose(database, redis_url, identity, kek, delivery)
}

pub(crate) fn compose(
    database: &(impl PostgresConnectionSource + ?Sized),
    redis_url: &str,
    identity: Arc<dyn IdentityWorkflow>,
    kek: Zeroizing<Vec<u8>>,
    delivery: Arc<dyn InvitationDelivery>,
) -> anyhow::Result<(Option<web::InvitationHttp>, Option<InvitationConsumer>)> {
    let protector = Arc::new(AesGcmInvitationProtector::new(kek.clone())?);
    let clock = Arc::new(SystemClock::new());
    let store = || PostgresInvitationStore::open(database).context("cannot open invitation store");
    // These budgets constrain failed/public attempts, never the number of accounts.
    let limiter = RedisInvitationLimiter::connect(
        redis_url,
        200,
        5,
        300,
        Duration::from_secs(1),
        Duration::from_secs(1),
    )?;
    let workflow = Arc::new(InvitationService::new(InvitationPorts {
        identity,
        users: Arc::new(PostgresUserRepository::open(database)?),
        store: Arc::new(store()?),
        protector: protector.clone(),
        secrets: Arc::new(AesGcmSecretProtector::new(kek)?),
        limiter: Arc::new(limiter),
        tokens: Arc::new(RandomResetTokenSource),
        digests: Arc::new(RingSha256Hasher::new()),
        passwords: Arc::new(Argon2idHasher::new()),
        totp: Arc::new(TotpRsProvider::new()),
        recovery: Arc::new(RandomRecoveryCodeGenerator),
        clock: clock.clone(),
    }));
    let worker = InvitationDeliveryWorker::new(Arc::new(store()?), protector, delivery, clock);
    Ok((
        Some(web::InvitationHttp { workflow }),
        Some(Arc::new(move || worker.run_once())),
    ))
}
