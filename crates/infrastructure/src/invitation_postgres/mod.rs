//! Recoverable invitation state on the existing audited PostgreSQL boundary.
mod delivery;
mod mutation;
mod read;
mod values;

use application::ApplicationError;
use postgres::Client;
use std::sync::{Mutex, MutexGuard};

pub struct PostgresInvitationStore {
    client: Mutex<Client>,
}
impl PostgresInvitationStore {
    pub fn open(
        source: &(impl crate::PostgresConnectionSource + ?Sized),
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::open(source)?),
        })
    }
    fn lock(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client.lock().map_err(|_| values::storage())
    }
}

use application::identity::invitations::*;
use domain::crypto::Sha256Digest;
impl InvitationStore for PostgresInvitationStore {
    fn issue(
        &self,
        a: InvitationActor,
        c: IssueInvitation,
    ) -> Result<InvitationSummary, ApplicationError> {
        self.write_issue(a, c)
    }
    fn resend(
        &self,
        a: InvitationActor,
        id: InvitationId,
        c: ResendInvitation,
    ) -> Result<InvitationSummary, ApplicationError> {
        self.write_resend(a, id, c)
    }
    fn list(
        &self,
        a: InvitationActor,
        q: InvitationQuery,
    ) -> Result<InvitationPage, ApplicationError> {
        self.read_list(a, q)
    }
    fn get(
        &self,
        a: InvitationActor,
        id: InvitationId,
    ) -> Result<InvitationSummary, ApplicationError> {
        self.read_get(a, id)
    }
    fn inspect(&self, d: Sha256Digest) -> Result<Option<InvitationCandidate>, ApplicationError> {
        self.read_candidate(d)
    }
    fn save_pending(&self, c: PrepareInvitation) -> Result<PendingInvitation, ApplicationError> {
        self.write_pending(c)
    }
    fn finish(&self, c: FinishInvitation) -> Result<InvitationReceipt, ApplicationError> {
        self.write_finish(c)
    }
    fn completed(
        &self,
        d: Sha256Digest,
        op: Uuid,
    ) -> Result<Option<InvitationCompletion>, ApplicationError> {
        self.read_completed(d, op)
    }
    fn claim_delivery(
        &self,
        c: InvitationDeliveryClaim,
    ) -> Result<Option<InvitationDeliveryLease>, ApplicationError> {
        self.claim_mail(c)
    }
    fn finish_delivery(&self, c: InvitationDeliveryFinish) -> Result<bool, ApplicationError> {
        self.finish_mail(c)
    }
}
