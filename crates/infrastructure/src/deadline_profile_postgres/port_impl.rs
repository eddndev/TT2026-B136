use super::{authorization, port, preparation, storage, PostgresDeadlineProfileStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{deadline_profiles::*, ApplicationError};
use domain::{clock::OffsetDateTime, identity::UserId};
impl DeadlineProfileStore for PostgresDeadlineProfileStore {
    fn list(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        query: DeadlineProfileQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineProfilePage, ApplicationError> {
        self.list_page(actor, collection, query, at)
    }
    fn get(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        id: DeadlineProfileId,
        revision: Option<DeadlineProfileRevision>,
        at: OffsetDateTime,
    ) -> Result<DeadlineProfileDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal =
            authorization::actor(&mut tx, actor, collection, false, self.hasher.as_ref())?;
        if !authorization::visible(&mut tx, collection, id, false)? {
            return Err(DeadlineProfileError::NotFound.into());
        }
        let detail = storage::detail(&mut tx, id, revision, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            "deadline_profile.read",
            &format!(
                "profile:{id}:revision:{}:sha256:{}",
                detail.revision.get(),
                detail.definition_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
    fn history(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        id: DeadlineProfileId,
        query: DeadlineProfileHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineProfileHistoryPage, ApplicationError> {
        self.history_page(actor, collection, id, query, at)
    }
    fn prepare(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        command: &DeadlineProfileCommand,
    ) -> Result<DeadlineProfilePreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorization::actor(&mut tx, actor, collection, true, self.hasher.as_ref())?;
        let result = preparation::load(&mut tx, collection, command, self.hasher.as_ref())?;
        tx.rollback().map_err(port)?;
        Ok(result)
    }
    fn commit(
        &self,
        actor: UserId,
        prepared: PreparedDeadlineProfileChange,
    ) -> Result<DeadlineProfileDetail, ApplicationError> {
        self.commit_change(actor, prepared)
    }
}
