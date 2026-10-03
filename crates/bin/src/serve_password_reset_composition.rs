//! Independently owned recovery adapters and their supervised request consumer.

use crate::{
    serve_password_reset_config::PasswordResetSettings,
    serve_password_reset_runtime::{PasswordResetConsumer, ResetRequestWork},
    serve_start::PasswordResetRuntime,
    serve_stop::Stop,
};
use anyhow::Context;
use application::{
    identity::password_reset::{PasswordResetDelivery, PasswordResetPorts, PasswordResetService},
    ApplicationError,
};
use infrastructure::{
    identity::{
        PostgresPasswordResetRepository, RandomResetTokenSource, RedisPasswordResetLimiter,
    },
    password_reset_email::{PasswordResetEmailConfiguration, ResendPasswordResetDelivery},
    Argon2idHasher, PostgresConnectionSource, RingSha256Hasher,
};
use std::{sync::Arc, time::Duration};
use web::{password_reset::PasswordResetHttp, HttpWorkBudget};
use zeroize::Zeroizing;

pub(crate) struct PasswordResetComponents {
    pub http: PasswordResetHttp,
    pub runtime: PasswordResetRuntime,
}

pub(crate) fn open(
    database: &(impl PostgresConnectionSource + ?Sized),
    redis_url: &str,
    settings: Option<PasswordResetSettings>,
    budget: HttpWorkBudget,
    stop: Arc<Stop>,
) -> anyhow::Result<Option<PasswordResetComponents>> {
    open_with_delivery_factory(
        database,
        redis_url,
        settings,
        budget,
        stop,
        |key, configuration, connect, total| {
            ResendPasswordResetDelivery::new(key, configuration, connect, total)
                .map(|delivery| Arc::new(delivery) as Arc<dyn PasswordResetDelivery>)
        },
    )
}

pub(crate) fn open_with_delivery_factory<F>(
    database: &(impl PostgresConnectionSource + ?Sized),
    redis_url: &str,
    settings: Option<PasswordResetSettings>,
    budget: HttpWorkBudget,
    stop: Arc<Stop>,
    delivery_factory: F,
) -> anyhow::Result<Option<PasswordResetComponents>>
where
    F: FnOnce(
        Zeroizing<String>,
        PasswordResetEmailConfiguration,
        Duration,
        Duration,
    ) -> Result<Arc<dyn PasswordResetDelivery>, ApplicationError>,
{
    let Some(settings) = settings else {
        return Ok(None);
    };
    let delivery = delivery_factory(
        settings.api_key,
        settings.configuration,
        settings.email_connect_timeout,
        settings.email_total_timeout,
    )
    .context("cannot configure recovery email transport")?;
    let limiter = Arc::new(
        RedisPasswordResetLimiter::connect(
            redis_url,
            settings.rates,
            settings.redis_connect_timeout,
            settings.redis_io_timeout,
        )
        .context("cannot configure recovery rate limits")?,
    );
    let tokens = Arc::new(RandomResetTokenSource);
    let digests = Arc::new(RingSha256Hasher::new());
    let passwords = Arc::new(Argon2idHasher::new());
    let service = |repository| {
        Arc::new(PasswordResetService::new(
            PasswordResetPorts {
                repository,
                delivery: delivery.clone(),
                limiter: limiter.clone(),
                tokens: tokens.clone(),
                digests: digests.clone(),
                passwords: passwords.clone(),
            },
            settings.policy,
        ))
    };
    let request = service(Arc::new(
        PostgresPasswordResetRepository::open(database)
            .context("cannot open recovery request repository")?,
    ));
    let completion = service(Arc::new(
        PostgresPasswordResetRepository::open(database)
            .context("cannot open recovery completion repository")?,
    ));
    let owner: ResetRequestWork = Arc::new(move |email| request.request(&email));
    let (admission, consumer) = PasswordResetConsumer::new(owner.clone(), budget, stop);
    Ok(Some(PasswordResetComponents {
        http: PasswordResetHttp {
            requests: Arc::new(admission),
            completion,
        },
        runtime: PasswordResetRuntime { owner, consumer },
    }))
}
