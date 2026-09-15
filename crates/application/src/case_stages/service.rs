use std::sync::Arc;

use domain::cases::CaseId;
use domain::clock::Clock;
use domain::identity::{Permission, UserId};

use super::{
    CaseStageChange, CaseStageDetail, CaseStageExpectation, CaseStagePage, CaseStageQuery,
    CaseStageRevision, CaseStageStore, CaseStageWorkflow, CurrentCaseStage,
    PreparedCaseStageChange, StageAdoption, StageSupportReadLimits, StageTransition,
};
use crate::documents::{DocumentFormatBatchValidator, DocumentProcessor, DocumentRecord};
use crate::{identity::IdentityWorkflow, ApplicationError};

pub struct CaseStageService {
    store: Arc<dyn CaseStageStore>,
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
    clock: Arc<dyn Clock + Send + Sync>,
    limits: StageSupportReadLimits,
}

impl CaseStageService {
    pub fn new(
        store: Arc<dyn CaseStageStore>,
        identity: Arc<dyn IdentityWorkflow>,
        processor: Arc<DocumentProcessor>,
        validator: Arc<dyn DocumentFormatBatchValidator>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            identity,
            processor,
            validator,
            clock,
            limits: StageSupportReadLimits::standard(),
        }
    }

    fn actor(&self, token: &str, permission: Permission) -> Result<UserId, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !actor.role.allows(permission) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor.id)
    }

    fn change(
        &self,
        token: &str,
        case_id: CaseId,
        expected: CaseStageExpectation,
        change: CaseStageChange,
    ) -> Result<CaseStageDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ManageCaseStage)?;
        if matches!(change, CaseStageChange::Adopt(_))
            && expected != CaseStageExpectation::Unregistered
        {
            return Err(ApplicationError::InvalidInput(
                "stage adoption expects no registered stage".into(),
            ));
        }
        let preparation = self
            .store
            .prepare(actor, case_id, expected, &change, &self.limits)?;
        check_head(case_id, &preparation.current, expected, &change)?;
        let records = ordered_records(&change, preparation.records)?;
        let formats = self.processor.validate_support_batch(
            &records,
            &self.limits,
            self.validator.as_ref(),
        )?;
        if self.actor(token, Permission::ManageCaseStage)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        let at = self.clock.now();
        change.validate_recording_at(at)?;
        self.store.commit(
            actor,
            case_id,
            expected,
            PreparedCaseStageChange::new(change, records, formats),
            at,
        )
    }
}

impl CaseStageWorkflow for CaseStageService {
    fn get(&self, token: &str, case_id: CaseId) -> Result<CaseStageDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ReadCaseStage)?;
        self.store.get(actor, case_id, self.clock.now())
    }
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        query: CaseStageQuery,
    ) -> Result<CaseStagePage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadCaseStage)?;
        self.store.history(actor, case_id, query, self.clock.now())
    }
    fn adopt(
        &self,
        token: &str,
        case_id: CaseId,
        expected: CaseStageExpectation,
        adoption: StageAdoption,
    ) -> Result<CaseStageDetail, ApplicationError> {
        self.change(token, case_id, expected, CaseStageChange::Adopt(adoption))
    }
    fn transition(
        &self,
        token: &str,
        case_id: CaseId,
        expected: CaseStageRevision,
        transition: StageTransition,
    ) -> Result<CaseStageDetail, ApplicationError> {
        self.change(
            token,
            case_id,
            CaseStageExpectation::Revision(expected),
            CaseStageChange::Transition(transition),
        )
    }
}

fn check_head(
    case_id: CaseId,
    current: &CurrentCaseStage,
    expected: CaseStageExpectation,
    change: &CaseStageChange,
) -> Result<(), ApplicationError> {
    if current
        .entry()
        .is_some_and(|entry| entry.case_id() != case_id)
    {
        return Err(ApplicationError::StoredCaseStageInconsistent(
            "prepared head belongs to another case".into(),
        ));
    }
    match change {
        CaseStageChange::Adopt(_) => {
            if current.entry().is_some() {
                return Err(ApplicationError::CaseStageConflict);
            }
        }
        CaseStageChange::Transition(_) => {
            let entry = current.entry().ok_or(ApplicationError::CaseStageRequired)?;
            if expected != CaseStageExpectation::Revision(entry.stage_revision()) {
                return Err(ApplicationError::CaseStageConflict);
            }
            if !entry.stage().permits(change.stage()) {
                return Err(ApplicationError::CaseStageTransitionRejected);
            }
            if entry.stage_revision().next().is_none() {
                return Err(ApplicationError::CaseStageRevisionExhausted);
            }
        }
    }
    Ok(())
}

fn ordered_records(
    change: &CaseStageChange,
    mut records: Vec<DocumentRecord>,
) -> Result<Vec<DocumentRecord>, ApplicationError> {
    let supports = change.supports();
    if records.len() != supports.len() {
        return Err(ApplicationError::StoredCaseStageInconsistent(
            "prepared support count differs from request".into(),
        ));
    }
    let mut ordered = Vec::with_capacity(supports.len());
    for support in supports {
        let reference = support.reference();
        let index = records
            .iter()
            .position(|record| record.id == reference.id && record.version == reference.version)
            .ok_or_else(|| {
                ApplicationError::StoredCaseStageInconsistent(
                    "prepared exact support is missing".into(),
                )
            })?;
        let record = records.remove(index);
        if record.digest != support.digest() {
            return Err(ApplicationError::StageSupportDigestMismatch);
        }
        ordered.push(record);
    }
    Ok(ordered)
}
