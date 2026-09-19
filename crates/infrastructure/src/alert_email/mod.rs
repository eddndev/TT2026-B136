//! Idempotent generic email submission through the Resend HTTP API.

#[cfg(test)]
mod tests;
mod validation;

use application::alerts::{AlertEmailMessage, AlertEmailOutcome, AlertEmailSender};
use reqwest::blocking::Client;
use std::io::Read;
use std::time::Duration;
use zeroize::Zeroizing;

/// Validate the same sender and fixed login destination used for every delivery.
pub fn validate_configuration(
    value: &application::alerts::AlertEmailConfiguration,
) -> Result<(), application::ApplicationError> {
    if !validation::address(&value.from_email) || !validation::login_url(&value.login_url) {
        return Err(application::ApplicationError::InvalidConfiguration(
            "invalid alert email sender or login URL".into(),
        ));
    }
    Ok(())
}

pub struct ResendAlertEmailSender {
    client: Client,
    endpoint: String,
    api_key: Zeroizing<String>,
}

impl ResendAlertEmailSender {
    pub fn new(api_key: String) -> Result<Self, application::ApplicationError> {
        if api_key.is_empty()
            || api_key.len() > 1024
            || !api_key.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(application::ApplicationError::InvalidConfiguration(
                "invalid alert email API key".into(),
            ));
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| {
                application::ApplicationError::InvalidConfiguration(
                    "cannot create alert email HTTP client".into(),
                )
            })?;
        Ok(Self {
            client,
            endpoint: "https://api.resend.com/emails".into(),
            api_key: Zeroizing::new(api_key),
        })
    }
}

impl AlertEmailSender for ResendAlertEmailSender {
    fn send(&self, message: &AlertEmailMessage) -> AlertEmailOutcome {
        if !validation::valid(message) {
            return AlertEmailOutcome::Permanent {
                code: "invalid_message".into(),
            };
        }
        let payload = serde_json::json!({
            "from": message.from_email,
            "to": [message.recipient_email],
            "subject": "Qadra: tienes avisos pendientes",
            "text": format!("Hay avisos en Qadra. Inicia sesion para consultarlos.\n\n{}", message.login_url),
        });
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(self.api_key.as_str())
            .header("Idempotency-Key", &message.idempotency_key)
            .json(&payload)
            .send();
        let Ok(response) = response else {
            return unknown("transport_uncertain");
        };
        let status = response.status().as_u16();
        let mut bytes = Vec::new();
        if response.take(8193).read_to_end(&mut bytes).is_err() || bytes.len() > 8192 {
            return unknown("response_uncertain");
        }
        let body: Option<serde_json::Value> = serde_json::from_slice(&bytes).ok();
        if status == 200 || status == 201 {
            let id = body
                .as_ref()
                .and_then(|body| body.get("id"))
                .and_then(|value| value.as_str());
            if let Some(id) = id
                .filter(|id| uuid::Uuid::parse_str(id).is_ok_and(|value| value.to_string() == *id))
            {
                return AlertEmailOutcome::Accepted {
                    provider_id: id.into(),
                };
            }
            return unknown("invalid_acceptance");
        }
        if status == 429 {
            return AlertEmailOutcome::Retryable {
                code: "rate_limited".into(),
            };
        }
        if status == 409 {
            return match body
                .as_ref()
                .and_then(|body| body.get("name"))
                .and_then(|value| value.as_str())
            {
                Some("concurrent_idempotent_requests") => AlertEmailOutcome::Retryable {
                    code: "concurrent_request".into(),
                },
                Some("invalid_idempotent_request") => AlertEmailOutcome::Permanent {
                    code: "idempotency_conflict".into(),
                },
                _ => unknown("provider_conflict"),
            };
        }
        if (400..500).contains(&status) && status != 408 {
            return AlertEmailOutcome::Permanent {
                code: "provider_rejected".into(),
            };
        }
        unknown("provider_uncertain")
    }
}

fn unknown(code: &str) -> AlertEmailOutcome {
    AlertEmailOutcome::Unknown { code: code.into() }
}
