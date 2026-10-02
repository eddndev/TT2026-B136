//! Bounded single-attempt delivery of password recovery capabilities through Resend.

mod configuration;
mod message;
mod response;

pub use configuration::PasswordResetEmailConfiguration;

use application::{
    identity::password_reset::{PasswordResetDelivery, ResetDeliveryOutcome, ResetEnvelope},
    ApplicationError,
};
use reqwest::blocking::Client;
use std::time::Duration;
use zeroize::Zeroizing;

const ENDPOINT: &str = "https://api.resend.com/emails";

/// Owns the configured secret without a public endpoint override or secret Debug output.
pub struct ResendPasswordResetDelivery {
    client: Client,
    endpoint: String,
    api_key: Zeroizing<String>,
    configuration: PasswordResetEmailConfiguration,
}

impl ResendPasswordResetDelivery {
    /// Timeouts must be positive, with connect <= 3s and connect <= total <= 10s.
    pub fn new(
        api_key: Zeroizing<String>,
        configuration: PasswordResetEmailConfiguration,
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
            configuration,
        })
    }

    #[cfg(test)]
    pub(crate) fn endpoint_for_test(&self) -> &str {
        &self.endpoint
    }

    #[cfg(test)]
    pub(crate) fn with_test_endpoint(mut self, endpoint: String) -> Result<Self, ApplicationError> {
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

impl PasswordResetDelivery for ResendPasswordResetDelivery {
    fn deliver(&self, envelope: ResetEnvelope) -> Result<ResetDeliveryOutcome, ApplicationError> {
        if !configuration::address(&envelope.email) {
            return Ok(ResetDeliveryOutcome::DefinitelyRejected);
        }
        let body = message::body(&self.configuration, &envelope)?;
        let idempotency_key = message::submission_id()?;
        let result = self
            .client
            .post(&self.endpoint)
            .bearer_auth(self.api_key.as_str())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header("Idempotency-Key", idempotency_key)
            .body(body)
            .send();
        Ok(match result {
            Ok(reply) => response::classify(reply),
            Err(_) => ResetDeliveryOutcome::Uncertain,
        })
    }
}
