use super::{
    DocumentIntegrityIncident, DocumentIntegrityIncidentId, DocumentIntegrityPage,
    DocumentIntegrityQuery, DocumentIntegrityStore, DocumentIntegrityWorkflow,
};
use crate::{
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::{clock::Clock, identity::Role};
use std::sync::Arc;

pub struct DocumentIntegrityService {
    store: Arc<dyn DocumentIntegrityStore>,
    identity: Arc<dyn IdentityWorkflow>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl DocumentIntegrityService {
    pub fn new(
        store: Arc<dyn DocumentIntegrityStore>,
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

impl DocumentIntegrityWorkflow for DocumentIntegrityService {
    fn list(
        &self,
        token: &str,
        query: DocumentIntegrityQuery,
    ) -> Result<DocumentIntegrityPage, ApplicationError> {
        let principal = self.owner(token)?;
        let started_at = self.clock.now();
        let page = self.store.list(principal.id, query, started_at)?;
        let returned_at = self.clock.now();
        super::validation::window(started_at, returned_at)?;
        super::validation::page(&page, query, returned_at)?;
        self.reauthenticate(token, &principal)?;
        Ok(page)
    }
    fn get(
        &self,
        token: &str,
        id: DocumentIntegrityIncidentId,
    ) -> Result<DocumentIntegrityIncident, ApplicationError> {
        let principal = self.owner(token)?;
        let started_at = self.clock.now();
        let record = self.store.get(principal.id, id, started_at)?;
        let returned_at = self.clock.now();
        super::validation::window(started_at, returned_at)?;
        super::validation::incident(&record, returned_at)?;
        if record.id != id {
            return Err(super::validation::inconsistent(
                "incident identity differs from the request",
            ));
        }
        self.reauthenticate(token, &principal)?;
        Ok(record)
    }
}
