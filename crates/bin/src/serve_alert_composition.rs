//! Share the durable alert store between HTTP and supervised consumers.

use crate::{serve_alert_config::AlertEmailSettings, serve_alert_runtime::AlertRuntimeConfig};
use anyhow::Context;
use application::{
    alerts::{
        AlertDeliveryStore, AlertEmailSender, AlertSchedulerStore, AlertService, AlertWorkflow,
    },
    identity::IdentityWorkflow,
};
use infrastructure::{
    alert_email::ResendAlertEmailSender, PostgresAlertStore, RingSha256Hasher, SystemClock,
};
use std::sync::Arc;

pub(crate) struct AlertConsumers {
    pub scheduler: Arc<dyn AlertSchedulerStore>,
    pub delivery: Arc<dyn AlertDeliveryStore>,
    pub sender: Option<Arc<dyn AlertEmailSender>>,
    pub config: AlertRuntimeConfig,
}

pub(crate) struct AlertComponents {
    pub workflow: Arc<dyn AlertWorkflow>,
    pub consumers: AlertConsumers,
}

pub(crate) fn open(
    database_url: &(impl infrastructure::PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
    email: Option<AlertEmailSettings>,
    config: AlertRuntimeConfig,
) -> anyhow::Result<AlertComponents> {
    let (configuration, sender) = match email {
        Some(settings) => {
            let (configuration, api_key) = settings.into_parts();
            let sender = ResendAlertEmailSender::new(api_key.to_string())
                .context("cannot initialize alert email transport")?;
            (
                Some(configuration),
                Some(Arc::new(sender) as Arc<dyn AlertEmailSender>),
            )
        }
        None => (None, None),
    };
    let store = Arc::new(
        PostgresAlertStore::open(
            database_url,
            Arc::new(RingSha256Hasher::new()),
            Arc::new(SystemClock::new()),
            configuration,
        )
        .context("cannot open PostgreSQL alert store")?,
    );
    Ok(AlertComponents {
        workflow: Arc::new(AlertService::new(store.clone(), identity)),
        consumers: AlertConsumers {
            scheduler: store.clone(),
            delivery: store,
            sender,
            config,
        },
    })
}
