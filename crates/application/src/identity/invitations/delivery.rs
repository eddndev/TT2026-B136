use domain::clock::OffsetDateTime;
use zeroize::Zeroizing;

use crate::identity::password_reset::ResetDeliveryOutcome;
use crate::ApplicationError;

use super::{InvitationId, Uuid};

/// One provider attempt with a stable delivery ID; retries belong to the worker.
/// An uncertain result never establishes that the capability was undelivered.
pub trait InvitationDelivery: Send + Sync {
    fn deliver(
        &self,
        lease: &InvitationDeliveryLease,
        token: Zeroizing<Vec<u8>>,
    ) -> Result<ResetDeliveryOutcome, ApplicationError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvitationDeliveryState {
    Pending,
    Sent,
    Uncertain,
    Rejected,
}

/// One bounded lease request. Its UUID fences an individual worker attempt.
/// provider_window_seconds bounds retries from the first claim, never from resend.
pub struct InvitationDeliveryClaim {
    pub lease_id: Uuid,
    pub at: OffsetDateTime,
    pub lease_seconds: u32,
    pub provider_window_seconds: u32,
}

/// Sensitive payload remains encrypted until the delivery adapter needs it.
/// delivery_id is the fixed provider idempotency key for this invitation revision.
#[derive(Clone)]
pub struct InvitationDeliveryLease {
    pub id: InvitationId,
    pub revision: u64,
    pub delivery_id: Uuid,
    pub generation: u64,
    pub lease_id: Uuid,
    pub email: String,
    pub protected_token: Vec<u8>,
    pub expires_at: OffsetDateTime,
    pub lease_until: OffsetDateTime,
    pub provider_deadline: OffsetDateTime,
}

pub struct InvitationDeliveryFinish {
    pub id: InvitationId,
    pub revision: u64,
    pub generation: u64,
    pub lease_id: Uuid,
    pub state: InvitationDeliveryState,
    pub retry_at: Option<OffsetDateTime>,
    pub at: OffsetDateTime,
}
