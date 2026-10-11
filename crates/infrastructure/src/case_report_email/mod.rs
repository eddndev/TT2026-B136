//! Bounded report email attempts using the complete persisted message.

mod configuration;
mod message;
#[cfg(test)]
mod tests;

pub use configuration::CaseReportEmailConfiguration;

use application::{
    case_reports::{CaseReportEmailDelivery, CaseReportEmailMessage, CaseReportEmailOutcome},
    identity::password_reset::ResetDeliveryOutcome,
    ApplicationError,
};
use reqwest::blocking::Client;
use std::time::Duration;
use uuid::Uuid;
use zeroize::Zeroizing;

const ENDPOINT: &str = "https://api.resend.com/emails";

/// One provider attempt without secret Debug output or a public endpoint override.
pub struct ResendCaseReportEmailDelivery {
    client: Client,
    endpoint: String,
    api_key: Zeroizing<String>,
}

impl ResendCaseReportEmailDelivery {
    /// Requires positive timeouts with connect <= 3s and connect <= total <= 10s.
    pub fn new(
        api_key: Zeroizing<String>,
        connect_timeout: Duration,
        total_timeout: Duration,
    ) -> Result<Self, ApplicationError> {
        if api_key.is_empty()
            || api_key.len() > 1024
            || !api_key.bytes().all(|byte| byte.is_ascii_graphic())
            || connect_timeout.is_zero()
            || total_timeout.is_zero()
            || connect_timeout > Duration::from_secs(3)
            || total_timeout > Duration::from_secs(10)
            || connect_timeout > total_timeout
        {
            return Err(configuration::invalid());
        }
        let client = Client::builder()
            .connect_timeout(connect_timeout)
            .timeout(total_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .no_proxy()
            .build()
            .map_err(|_| configuration::invalid())?;
        Ok(Self {
            client,
            endpoint: ENDPOINT.into(),
            api_key,
        })
    }

    #[cfg(test)]
    fn endpoint_for_test(&self) -> &str {
        &self.endpoint
    }

    #[cfg(test)]
    fn with_test_endpoint(mut self, endpoint: String) -> Result<Self, ApplicationError> {
        let url = reqwest::Url::parse(&endpoint).map_err(|_| configuration::invalid())?;
        if url.scheme() != "http"
            || !url.host_str().is_some_and(|host| {
                host.parse::<std::net::IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
            })
            || url.port().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/emails"
        {
            return Err(configuration::invalid());
        }
        self.endpoint = endpoint;
        Ok(self)
    }
}

impl CaseReportEmailDelivery for ResendCaseReportEmailDelivery {
    fn deliver(
        &self,
        delivery_id: Uuid,
        message: &CaseReportEmailMessage,
    ) -> Result<CaseReportEmailOutcome, ApplicationError> {
        if delivery_id.is_nil() || !message::valid(message) {
            return Ok(CaseReportEmailOutcome::DefinitelyRejected);
        }
        let result = self
            .client
            .post(&self.endpoint)
            .bearer_auth(self.api_key.as_str())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                "Idempotency-Key",
                format!("case-report-ready:{delivery_id}"),
            )
            .body(message::body(message)?)
            .send();
        Ok(match result {
            Ok(reply) => match crate::password_reset_email::response::classify(reply) {
                ResetDeliveryOutcome::Accepted => CaseReportEmailOutcome::Accepted,
                ResetDeliveryOutcome::DefinitelyRejected => {
                    CaseReportEmailOutcome::DefinitelyRejected
                }
                ResetDeliveryOutcome::Uncertain => CaseReportEmailOutcome::Uncertain,
            },
            Err(_) => CaseReportEmailOutcome::Uncertain,
        })
    }
}
