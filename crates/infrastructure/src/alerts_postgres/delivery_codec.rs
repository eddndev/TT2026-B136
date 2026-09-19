use super::super::{codec, stored};
use super::state::{Claim, State, MAX_ATTEMPTS};
use application::{alerts::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::Row;
use serde_json::{json, Value};
use time::Duration;

pub(super) fn message(value: &AlertEmailMessage) -> Value {
    json!({"key":value.idempotency_key,"from":value.from_email,"to":value.recipient_email,"login":value.login_url,"template":"generic_login_v1"})
}
fn read_message(value: &Value) -> Result<AlertEmailMessage, ApplicationError> {
    if codec::text(&value["template"])? != "generic_login_v1" {
        return Err(stored("invalid delivery template"));
    }
    let result = AlertEmailMessage {
        idempotency_key: codec::text(&value["key"])?,
        from_email: codec::text(&value["from"])?,
        recipient_email: codec::text(&value["to"])?,
        login_url: codec::text(&value["login"])?,
        template: AlertEmailTemplate::GenericLoginV1,
    };
    crate::alert_email::validate_configuration(&AlertEmailConfiguration {
        from_email: result.from_email.clone(),
        login_url: result.login_url.clone(),
    })
    .map_err(|_| stored("invalid frozen delivery configuration"))?;
    if result.recipient_email.len() > 320
        || result
            .recipient_email
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
        || !result
            .recipient_email
            .split_once('@')
            .is_some_and(|(local, host)| {
                !local.is_empty() && !host.is_empty() && !host.contains('@')
            })
    {
        return Err(stored("invalid frozen delivery recipient"));
    }
    Ok(result)
}
pub(super) fn outcome(value: &AlertEmailOutcome) -> Value {
    match value {
        AlertEmailOutcome::Accepted { provider_id } => json!(["accepted", provider_id]),
        AlertEmailOutcome::Permanent { code } => json!(["permanent", code]),
        AlertEmailOutcome::Retryable { code } => json!(["retryable", code]),
        AlertEmailOutcome::Unknown { code } => json!(["unknown", code]),
    }
}
pub(super) fn validate_outcome(value: &AlertEmailOutcome) -> Result<(), ApplicationError> {
    let text = match value {
        AlertEmailOutcome::Accepted { provider_id } => provider_id,
        AlertEmailOutcome::Permanent { code }
        | AlertEmailOutcome::Retryable { code }
        | AlertEmailOutcome::Unknown { code } => code,
    };
    if text.is_empty() || text.len() > 256 || !text.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(stored("invalid delivery outcome"));
    }
    Ok(())
}
fn read_outcome(value: &Value) -> Result<AlertEmailOutcome, ApplicationError> {
    let text = codec::text(&value[1])?;
    let result = match codec::text(&value[0])?.as_str() {
        "accepted" => AlertEmailOutcome::Accepted { provider_id: text },
        "permanent" => AlertEmailOutcome::Permanent { code: text },
        "retryable" => AlertEmailOutcome::Retryable { code: text },
        "unknown" => AlertEmailOutcome::Unknown { code: text },
        _ => return Err(stored("invalid delivery outcome")),
    };
    validate_outcome(&result)?;
    Ok(result)
}
pub(super) fn decode(row: &Row, hasher: &dyn DocumentHasher) -> Result<State, ApplicationError> {
    let value = codec::read(row, hasher)?;
    let state = State {
        id: codec::uuid(&value["id"])?,
        alert: AlertId::from_uuid(codec::uuid(&value["alert"])?),
        sequence: codec::integer(&value["sequence"])?,
        status: codec::text(&value["status"])?,
        attempt: u32::try_from(codec::integer(&value["attempt"])?).map_err(stored)?,
        at: codec::time(&value["at"])?,
        next: codec::optional_time(&value["next"])?,
        first: codec::optional_time(&value["first"])?,
        claim: if value["claim"].is_null() {
            None
        } else {
            Some(Claim {
                id: codec::uuid(&value["claim"]["id"])?,
                at: codec::time(&value["claim"]["at"])?,
                until: codec::time(&value["claim"]["until"])?,
            })
        },
        message: if value["message"].is_null() {
            None
        } else {
            Some(read_message(&value["message"])?)
        },
        outcome: if value["outcome"].is_null() {
            None
        } else {
            Some(read_outcome(&value["outcome"])?)
        },
        uncertain: codec::boolean(&value["uncertain"])?,
    };
    state.email_status()?;
    if state.encode() != value
        || state.sequence < 0
        || state.sequence > 32
        || state.attempt > MAX_ATTEMPTS
        || state.next.is_some_and(|next| next < state.at)
        || state.first.is_some_and(|first| first > state.at)
        || (state.attempt == 0)
            != (state.first.is_none() && state.message.is_none() && state.claim.is_none())
    {
        return Err(stored("delivery state evidence differs"));
    }
    if let Some(claim) = &state.claim {
        if Some(claim.until) != super::state::after(claim.at, Duration::seconds(120))
            || claim.at > state.at
            || state.first.is_none_or(|first| first > claim.at)
        {
            return Err(stored("delivery lease differs"));
        }
    }
    if let Some(message) = &state.message {
        if message.idempotency_key != format!("alert-email:{}", state.id) {
            return Err(stored("delivery idempotency key differs"));
        }
    }
    if state.status == "sending"
        && (state.claim.is_none()
            || state.outcome.is_some()
            || state.next != state.claim.as_ref().map(|c| c.until))
    {
        return Err(stored("sending delivery lacks exact lease"));
    }
    if state.status == "accepted"
        && (!matches!(state.outcome, Some(AlertEmailOutcome::Accepted { .. }))
            || state.next.is_some()
            || state.uncertain)
    {
        return Err(stored("accepted delivery differs"));
    }
    if matches!(state.status.as_str(), "failed" | "cancelled" | "disabled") && state.next.is_some()
    {
        return Err(stored("terminal delivery retries"));
    }
    if state.status == "unknown" && !state.uncertain {
        return Err(stored("unknown delivery lost uncertainty"));
    }
    if matches!(
        state.status.as_str(),
        "pending" | "failed" | "cancelled" | "disabled"
    ) && state.uncertain
    {
        return Err(stored("delivery uncertainty was relabelled"));
    }
    Ok(state)
}
