use super::{workflow_evidence as evidence, *};
use crate::{identity::IdentityWorkflow, ApplicationError};
use domain::{cases::CaseId, clock::Clock, crypto::DocumentHasher, identity::Role};
use std::sync::Arc;

pub struct PrecautionaryContextReadService {
    store: Arc<dyn PrecautionaryContextReadStore>,
    identity: Arc<dyn IdentityWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PrecautionaryContextReadService {
    pub fn new(
        store: Arc<dyn PrecautionaryContextReadStore>,
        identity: Arc<dyn IdentityWorkflow>,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            identity,
            hasher,
            clock,
        }
    }
}
impl PrecautionaryContextReadWorkflow for PrecautionaryContextReadService {
    fn get(&self, token: &str, case_id: CaseId) -> Result<PrecautionaryContext, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if actor.role == Role::Client {
            return Err(ApplicationError::PermissionDenied);
        }
        let started = self.clock.now();
        evidence::clock(started)?;
        let context = self.store.get(&actor, case_id)?;
        let material = context.material();
        if material.case_id != case_id {
            return Err(evidence::invalid("context read case differs"));
        }
        PrecautionaryContext::new(self.hasher.as_ref(), material.clone())
            .map_err(|error| evidence::invalid(&error.to_string()))?;
        let floor = started
            .max(material.administration.changed_at)
            .max(material.stage.recorded_at())
            .max(material.stage_administration.changed_at);
        if self.identity.authenticate(token)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        let at = self.clock.now();
        evidence::clock(at)?;
        if at < floor {
            return Err(evidence::invalid(
                "context read clock precedes its observation or sources",
            ));
        }
        Ok(context)
    }
}
