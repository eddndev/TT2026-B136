use super::{record_workflow_evidence as evidence, workflow_evidence, *};
use crate::{
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::DocumentHasher,
    identity::Role,
};

pub(super) fn actor(
    identity: &dyn IdentityWorkflow,
    token: &str,
) -> Result<Principal, ApplicationError> {
    let actor = identity.authenticate(token)?;
    if !matches!(actor.role, Role::Owner | Role::Litigator) {
        return Err(ApplicationError::PermissionDenied);
    }
    Ok(actor)
}

pub(super) fn reauthenticate(
    identity: &dyn IdentityWorkflow,
    token: &str,
    expected: &Principal,
) -> Result<(), ApplicationError> {
    if actor(identity, token)? != *expected {
        return Err(ApplicationError::InvalidSession);
    }
    Ok(())
}

pub(super) fn observation(
    clock: &dyn Clock,
    lower: Option<OffsetDateTime>,
) -> Result<OffsetDateTime, ApplicationError> {
    let now = clock.now();
    workflow_evidence::clock(now)?;
    if lower.is_some_and(|lower| now < lower) {
        return Err(workflow_evidence::invalid("service clock regressed"));
    }
    Ok(now)
}

pub(super) fn confirmation(
    review: &PrecautionaryHearingReview,
    expected: PrecautionaryHearingConfirmation,
) -> Result<(), ApplicationError> {
    if review.submission_digest != expected.submission_digest {
        return Err(PrecautionaryHearingError::SubmissionMismatch.into());
    }
    if review.review_digest != expected.review_digest {
        return Err(PrecautionaryHearingError::ReviewMismatch.into());
    }
    Ok(())
}

pub(super) fn replay(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: &PrecautionaryHearingCommand,
    result: &PrecautionaryHearingRecordStoredOperation,
) -> Result<(), ApplicationError> {
    evidence::operation(hasher, result)?;
    let review = &result.capture.review;
    if review.case_id != case_id || review.actor.id != actor.id || review.command != *command {
        return Err(PrecautionaryHearingError::OperationConflict.into());
    }
    Ok(())
}

pub(super) struct CommitExpectation {
    review: PrecautionaryHearingReview,
    captures: Vec<PrecautionaryHearingCapture>,
    origin: Option<PrecautionaryHearingOrigin>,
    records: crate::precautionary_measures::MeasureDecisionRecordHistoryEvidence,
}
impl CommitExpectation {
    pub fn new(prepared: &PreparedPrecautionaryHearingRecord) -> Self {
        let history = prepared.material().history.as_ref();
        Self {
            review: prepared.review().clone(),
            captures: history.map(|h| h.captures.clone()).unwrap_or_default(),
            origin: history.map(|h| h.origin.clone()),
            records: prepared.merged.clone(),
        }
    }

    pub fn matches(
        &self,
        result: &PrecautionaryHearingRecordStoredOperation,
    ) -> Result<(), ApplicationError> {
        let (_, prefix) = result
            .history
            .captures
            .split_last()
            .ok_or_else(|| workflow_evidence::invalid("returned operation has no capture"))?;
        if result.capture.review != self.review
            || prefix != self.captures
            || self
                .origin
                .as_ref()
                .is_some_and(|o| *o != result.history.origin)
            || evidence::merge(&self.records, &result.history.record_history)? != self.records
        {
            return Err(workflow_evidence::invalid(
                "committed operation differs from complete reviewed preparation",
            ));
        }
        Ok(())
    }
}
