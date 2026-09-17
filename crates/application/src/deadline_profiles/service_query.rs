use super::{
    catalog_validation::{validate_history, validate_page},
    receipt::inconsistent,
    *,
};
use crate::ApplicationError;
use domain::{crypto::Sha256Digest, identity::Permission};

impl DeadlineProfileWorkflow for DeadlineProfileService {
    fn list(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        query: DeadlineProfileQuery,
    ) -> Result<DeadlineProfilePage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDeadlineProfile)?;
        let page = self
            .store
            .list(actor.0, collection, query.clone(), self.clock.now())?;
        validate_page(collection, &page, &query)?;
        self.same_actor(token, actor, Permission::ReadDeadlineProfile)?;
        Ok(page)
    }
    fn get(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        id: DeadlineProfileId,
        revision: Option<DeadlineProfileRevision>,
    ) -> Result<DeadlineProfileDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDeadlineProfile)?;
        let result = self
            .store
            .get(actor.0, collection, id, revision, self.clock.now())?;
        deadline_profile_receipt_matches(self.hasher.as_ref(), &result)?;
        if result.id != id
            || revision.is_some_and(|r| result.revision != r)
            || !collection.includes(result.definition.scope())
        {
            return Err(inconsistent(
                "profile detail differs from requested scope or revision",
            ));
        }
        self.same_actor(token, actor, Permission::ReadDeadlineProfile)?;
        Ok(result)
    }
    fn history(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        id: DeadlineProfileId,
        query: DeadlineProfileHistoryQuery,
    ) -> Result<DeadlineProfileHistoryPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDeadlineProfile)?;
        let result = self
            .store
            .history(actor.0, collection, id, query, self.clock.now())?;
        validate_history(self.hasher.as_ref(), collection, id, &result, query)?;
        self.same_actor(token, actor, Permission::ReadDeadlineProfile)?;
        Ok(result)
    }
    fn prepare(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        command: DeadlineProfileCommand,
    ) -> Result<DeadlineProfileDraft, ApplicationError> {
        self.prepare_command(token, collection, command)
    }
    fn submit(
        &self,
        token: &str,
        collection: DeadlineProfileCollection,
        command: DeadlineProfileCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<DeadlineProfileDetail, ApplicationError> {
        self.submit_command(token, collection, command, expected_submission_digest)
    }
}
