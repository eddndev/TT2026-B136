use super::{AlertClaimId, AlertDeliveryId, AlertId, AlertSubject};
use domain::clock::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertEmailConfiguration {
    pub from_email: String,
    pub login_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertEmailTemplate {
    GenericLoginV1,
}
/// Frozen provider input. A retry preserves every field, including the sender.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertEmailMessage {
    pub idempotency_key: String,
    pub from_email: String,
    pub recipient_email: String,
    pub login_url: String,
    pub template: AlertEmailTemplate,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertEmailOutcome {
    Accepted { provider_id: String },
    Permanent { code: String },
    Retryable { code: String },
    Unknown { code: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertDeliveryClaim {
    pub delivery_id: AlertDeliveryId,
    pub alert_id: AlertId,
    pub claim_id: AlertClaimId,
    pub attempt: u32,
    pub claimed_at: OffsetDateTime,
    pub lease_until: OffsetDateTime,
    pub first_attempt_at: OffsetDateTime,
    pub message: AlertEmailMessage,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertDeliveryCompletion {
    pub delivery_id: AlertDeliveryId,
    pub claim_id: AlertClaimId,
    pub attempt: u32,
    pub outcome: AlertEmailOutcome,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSchedulerRun {
    Idle,
    Reconciled {
        subject: AlertSubject,
        scheduled: u32,
        superseded: u32,
    },
    Activated {
        alert_id: AlertId,
    },
}
