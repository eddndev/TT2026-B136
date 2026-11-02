//! The periodic closer of exhausted requests. Each cycle writes the audited
//! `expired` event and its inbox notices for at most [`EXPIRY_BATCH`]
//! requests whose validity is exhausted and that no event has closed. It
//! does not depend on e-mail delivery: notices stay in the inbox until a
//! separate consumer sends them. Signing never waits for this closer,
//! because an exhausted request is no longer open.

use std::sync::Arc;

use domain::clock::Clock;
use domain::document_signing::Uuid;

use super::{
    error::ContractWorkflowError,
    model::EventKind,
    notices::{self, NoticeSettings},
    port::{ContractCommand, ContractStore, ContractWrite},
    state::unclosed_request,
};
use crate::ApplicationError;

/// Most requests one cycle closes.
pub const EXPIRY_BATCH: usize = 20;

pub struct ContractExpiryService {
    store: Arc<dyn ContractStore>,
    clock: Arc<dyn Clock + Send + Sync>,
    notices: NoticeSettings,
}

impl ContractExpiryService {
    pub fn new(
        store: Arc<dyn ContractStore>,
        clock: Arc<dyn Clock + Send + Sync>,
        notices: NoticeSettings,
    ) -> Self {
        Self {
            store,
            clock,
            notices,
        }
    }

    /// Closes one batch and returns how many requests it closed. A request
    /// closed concurrently by another writer is skipped, not retried.
    pub fn run_once(&self) -> Result<usize, ApplicationError> {
        let now = self.clock.now();
        let mut closed = 0;
        for snapshot in self.store.exhausted(now.unix_timestamp(), EXPIRY_BATCH)? {
            let Some(request) = unclosed_request(&snapshot)
                .filter(|request| now.unix_timestamp() >= request.expires_at())
            else {
                continue;
            };
            let revision = snapshot
                .revision()
                .checked_add(1)
                .ok_or(ContractWorkflowError::Inconsistent)?;
            let command = ContractCommand {
                actor: None,
                operation: Uuid::new_v4(),
                revision,
                at: now,
                write: ContractWrite::Close {
                    contract: snapshot.root.id,
                    request: request.id(),
                    kind: EventKind::Expired,
                    reason: None,
                },
                notices: notices::notices(
                    &self.notices,
                    EventKind::Expired,
                    request.request.fields(),
                    &snapshot.root.title,
                ),
            };
            match self.store.commit(command) {
                Ok(_) => closed += 1,
                Err(ApplicationError::Contract(
                    ContractWorkflowError::RevisionConflict | ContractWorkflowError::RequestClosed,
                )) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(closed)
    }
}
