use super::*;
use crate::{documents::StageSupportReadLimits, identity::Principal, ApplicationError};
use domain::cases::CaseId;

pub trait MeasureDecisionRecordStore: Send + Sync {
    /// Reauthorize the complete current principal and case membership before lookup.
    /// Exact replay keeps its original family and remains readable on closed cases.
    /// Fresh work requires active exact context and current Valid affected heads.
    /// Resolve actual whole owners and historical sources, and verify each anchor's
    /// durable origin and complete hearing prefix before returning its material.
    fn prepare(
        &self,
        actor: &Principal,
        case_id: CaseId,
        command: &MeasureDecisionCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<MeasureDecisionRecordPreparation, ApplicationError>;

    /// Reauthorize and reload exact heads, context, sources, anchors and admitted
    /// encrypted support under the audit lock. Append the genuine V2 group and
    /// original audit atomically. Return an exact raced V2 receipt unchanged;
    /// a raced operation of another family is a conflict, never a conversion.
    fn commit(
        &self,
        actor: &Principal,
        case_id: CaseId,
        prepared: PreparedMeasureDecisionRecord,
    ) -> Result<MeasureDecisionRecordStoredOperation, ApplicationError>;
}

pub trait MeasureDecisionRecordWorkflow: Send + Sync {
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureDecisionCommand,
    ) -> Result<MeasureDecisionRecordReview, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: MeasureDecisionCommand,
        confirmation: MeasureDecisionConfirmation,
    ) -> Result<MeasureDecisionRecordReceipt, ApplicationError>;
}
