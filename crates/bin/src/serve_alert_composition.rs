//! Share the durable alert store between HTTP and supervised consumers.

use crate::{
    serve_alert_config::AlertEmailSettings, serve_alert_runtime::AlertRuntimeConfig,
    serve_args::ServeArgs,
};
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
use std::{env::VarError, sync::Arc};

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

pub(crate) fn email_settings(args: &ServeArgs) -> anyhow::Result<Option<AlertEmailSettings>> {
    let api_key = match std::env::var("RESEND_API_KEY") {
        Ok(value) => Some(value),
        Err(VarError::NotPresent) => None,
        Err(VarError::NotUnicode(_)) => anyhow::bail!("RESEND_API_KEY must be valid text"),
    };
    AlertEmailSettings::from_optional(
        api_key,
        args.alert_email_from.clone(),
        args.alert_login_url.clone(),
    )
    .context("cannot configure alert email")
}

pub(crate) fn open(
    database_url: &str,
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
