//! Share the report store with an explicitly activated email consumer.
use crate::{
    serve_report_email_config::ReportEmailSettings, serve_report_email_runtime::ReportEmailConsumer,
};
use application::case_reports::{CaseReportEmailStore, CaseReportEmailWorker};
use domain::clock::Clock;
use infrastructure::ResendCaseReportEmailDelivery;
use std::{sync::Arc, time::Duration};

pub(crate) fn open(
    settings: Option<ReportEmailSettings>,
    store: Arc<dyn CaseReportEmailStore>,
    clock: Arc<dyn Clock + Send + Sync>,
) -> anyhow::Result<Option<ReportEmailConsumer>> {
    let Some(settings) = settings else {
        return Ok(None);
    };
    let template = settings.configuration.template();
    let delivery = Arc::new(ResendCaseReportEmailDelivery::new(
        settings.api_key,
        Duration::from_secs(2),
        Duration::from_secs(5),
    )?);
    let worker = CaseReportEmailWorker::new(store, delivery, template, clock);
    Ok(Some(Arc::new(move || worker.run_once())))
}
