use super::{
    validation, CaseId, CaseMemberPage, CaseMemberQuery, MemberError, MemberStore, MemberWorkflow,
    Role, UserAccessChange, UserId, UserPage, UserQuery, UserSummary,
};
use crate::{
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::clock::Clock;
use std::sync::Arc;

pub struct MemberService {
    store: Arc<dyn MemberStore>,
    identity: Arc<dyn IdentityWorkflow>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl MemberService {
    pub fn new(
        store: Arc<dyn MemberStore>,
        identity: Arc<dyn IdentityWorkflow>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            identity,
            clock,
        }
    }

    fn owner(&self, token: &str) -> Result<Principal, ApplicationError> {
        let principal = self.identity.authenticate(token)?;
        if principal.role != Role::Owner {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(principal)
    }

    fn reauthenticate(&self, token: &str, expected: &Principal) -> Result<(), ApplicationError> {
        if self.identity.authenticate(token)? != *expected {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
}

impl MemberWorkflow for MemberService {
    fn list(&self, token: &str, query: UserQuery) -> Result<UserPage, ApplicationError> {
        let actor = self.owner(token)?;
        let start = self.clock.now();
        let page = self.store.list(actor.id, query.clone(), start)?;
        validation::window(start, self.clock.now())?;
        validation::users(&page, &query)?;
        self.reauthenticate(token, &actor)?;
        Ok(page)
    }
    fn get(&self, token: &str, id: UserId) -> Result<UserSummary, ApplicationError> {
        let actor = self.owner(token)?;
        let start = self.clock.now();
        let user = self.store.get(actor.id, id, start)?;
        validation::window(start, self.clock.now())?;
        super::validate_user_summary(&user)?;
        if user.id != id {
            return Err(MemberError::Stored("user detail differs from its identifier").into());
        }
        self.reauthenticate(token, &actor)?;
        Ok(user)
    }
    fn change_access(
        &self,
        token: &str,
        id: UserId,
        change: UserAccessChange,
    ) -> Result<UserSummary, ApplicationError> {
        let actor = self.owner(token)?;
        self.reauthenticate(token, &actor)?;
        let user = self
            .store
            .change_access(actor.id, id, change, self.clock.now())?;
        validation::access(&user, id, change)?;
        if id == actor.id && user.email != actor.email {
            return Err(
                MemberError::Stored("own account email changed during access update").into(),
            );
        }
        // A successful self-change invalidates this session; the receipt is final.
        Ok(user)
    }
    fn list_case_members(
        &self,
        token: &str,
        case_id: CaseId,
        query: CaseMemberQuery,
    ) -> Result<CaseMemberPage, ApplicationError> {
        let actor = self.owner(token)?;
        if query.case_id() != case_id {
            return Err(MemberError::Invalid("member query belongs to a different case").into());
        }
        let start = self.clock.now();
        let page = self
            .store
            .list_case_members(actor.id, case_id, query.clone(), start)?;
        let returned_at = self.clock.now();
        validation::window(start, returned_at)?;
        validation::case_members(&page, &query, returned_at)?;
        self.reauthenticate(token, &actor)?;
        Ok(page)
    }
}
