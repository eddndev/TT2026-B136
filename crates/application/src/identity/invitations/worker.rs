//! A single leased invitation delivery attempt with durable outcome recording.

use std::sync::Arc;

use domain::clock::{Clock, OffsetDateTime};
use time::Duration;

use crate::identity::password_reset::ResetDeliveryOutcome;
use crate::ApplicationError;

use super::{
    InvitationDelivery, InvitationDeliveryClaim, InvitationDeliveryFinish, InvitationDeliveryLease,
    InvitationDeliveryState, InvitationProtectionContext, InvitationProtector, InvitationPurpose,
    InvitationStore, Uuid,
};

const LEASE_SECONDS: u32 = 30;
const PROVIDER_WINDOW_SECONDS: u32 = 86_400;
const RETRY_DELAY_SECONDS: i64 = 60;

/// Recorded confirms a fenced store update, independently of provider acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvitationDeliveryRun {
    Idle,
    Stale,
    Recorded(InvitationDeliveryState),
}

/// Claims at most one row and delegates any scheduling to its caller.
pub struct InvitationDeliveryWorker {
    store: Arc<dyn InvitationStore>,
    protector: Arc<dyn InvitationProtector>,
    delivery: Arc<dyn InvitationDelivery>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl InvitationDeliveryWorker {
    pub fn new(
        store: Arc<dyn InvitationStore>,
        protector: Arc<dyn InvitationProtector>,
        delivery: Arc<dyn InvitationDelivery>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            protector,
            delivery,
            clock,
        }
    }

    /// Sends once within the captured lease; the store fences concurrent changes.
    pub fn run_once(&self) -> Result<InvitationDeliveryRun, ApplicationError> {
        let lease_id = Uuid::new_v4();
        let lease = self
            .store
            .claim_delivery(InvitationDeliveryClaim {
                lease_id,
                at: self.clock.now(),
                lease_seconds: LEASE_SECONDS,
                provider_window_seconds: PROVIDER_WINDOW_SECONDS,
            })
            .map_err(|_| unavailable("invitation delivery claim failed"))?;
        let Some(lease) = lease else {
            return Ok(InvitationDeliveryRun::Idle);
        };
        if lease.lease_id != lease_id || !live(&lease, self.clock.now()) {
            return Ok(InvitationDeliveryRun::Stale);
        }
        let token = self.protector.expose(
            InvitationProtectionContext {
                purpose: InvitationPurpose::Delivery,
                id: lease.id,
                revision: lease.revision,
            },
            &lease.protected_token,
        );
        if !live(&lease, self.clock.now()) {
            return Ok(InvitationDeliveryRun::Stale);
        }
        let outcome = match token {
            Ok(token) if token.len() == 32 => self
                .delivery
                .deliver(&lease, token)
                .unwrap_or(ResetDeliveryOutcome::Uncertain),
            _ => ResetDeliveryOutcome::Uncertain,
        };
        self.record(&lease, outcome)
    }

    fn record(
        &self,
        lease: &InvitationDeliveryLease,
        outcome: ResetDeliveryOutcome,
    ) -> Result<InvitationDeliveryRun, ApplicationError> {
        let state = match outcome {
            ResetDeliveryOutcome::Accepted => InvitationDeliveryState::Sent,
            ResetDeliveryOutcome::DefinitelyRejected => InvitationDeliveryState::Rejected,
            ResetDeliveryOutcome::Uncertain => InvitationDeliveryState::Uncertain,
        };
        let at = self.clock.now();
        let retry_at = if state == InvitationDeliveryState::Uncertain {
            at.checked_add(Duration::seconds(RETRY_DELAY_SECONDS))
                .filter(|retry| *retry < lease.provider_deadline && *retry < lease.expires_at)
        } else {
            None
        };
        let recorded = self
            .store
            .finish_delivery(InvitationDeliveryFinish {
                id: lease.id,
                revision: lease.revision,
                generation: lease.generation,
                lease_id: lease.lease_id,
                state,
                retry_at,
                at,
            })
            .map_err(|_| unavailable("invitation delivery outcome could not be recorded"))?;
        Ok(if recorded {
            InvitationDeliveryRun::Recorded(state)
        } else {
            InvitationDeliveryRun::Stale
        })
    }
}

fn live(lease: &InvitationDeliveryLease, at: OffsetDateTime) -> bool {
    at < lease.lease_until && at < lease.provider_deadline && at < lease.expires_at
}

fn unavailable(message: &str) -> ApplicationError {
    ApplicationError::Port(message.into())
}
