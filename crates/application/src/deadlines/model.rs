use super::*;
use crate::{
    cases::CurrentCaseAdministration,
    deadline_evaluations::{DeadlineEvaluationInput, DeadlineEvaluationRecord},
    deadline_inputs::DeadlineInputMaterial,
    deadline_profiles::{DeadlineProfileDetail, DeadlineProfileId, DeadlineProfileRevision},
};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    identity::{Role, UserId},
    procedural_facts::{FactLabel, FactText},
    procedural_time::DeclaredProceduralTime,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineProfileRef {
    pub id: DeadlineProfileId,
    pub revision: DeadlineProfileRevision,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineDefinition {
    pub title: FactLabel,
    pub profile: DeadlineProfileRef,
    pub input: DeadlineEvaluationInput,
    pub responsible: UserId,
}
/// A declared action does not establish that a legal filing was valid or timely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineAttention {
    Pending,
    Recorded {
        occurred_at: DeclaredProceduralTime,
        statement: FactText,
        locator: FactLabel,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineAction {
    Register,
    Correct,
    SetAttention,
    Retire,
    Reevaluate,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineChange {
    Register {
        definition: DeadlineDefinition,
    },
    Correct {
        expected_revision: DeadlineRevision,
        definition: DeadlineDefinition,
        reason: FactText,
    },
    SetAttention {
        expected_revision: DeadlineRevision,
        attention: DeadlineAttention,
        reason: FactText,
    },
    Retire {
        expected_revision: DeadlineRevision,
        reason: FactText,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineCommand {
    pub operation_id: DeadlineOperationId,
    pub deadline_id: DeadlineId,
    pub change: DeadlineChange,
}
pub use crate::deadline_reevaluation::TrackedAuthor as DeadlineActorSnapshot;
/// Captured identity only; assignment never grants access to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineResponsibleSnapshot {
    pub id: UserId,
    pub email: String,
    pub role: Role,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineResolvedInputs {
    pub profile: DeadlineProfileDetail,
    pub profile_head: DeadlineProfileDetail,
    pub material: DeadlineInputMaterial,
}
/// The result is stored, including its trace. Reading history never reruns arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineCalculation {
    pub profile: DeadlineProfileDetail,
    pub material: DeadlineInputMaterial,
    pub result: DeadlineEvaluationRecord,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineReceipt {
    pub version: DeadlineReceiptVersion,
    pub operation_id: DeadlineOperationId,
    pub action: DeadlineAction,
    pub expected_revision: u32,
    pub review_digest: Sha256Digest,
    pub capture_digest: Sha256Digest,
    pub submission_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineDetail {
    pub id: DeadlineId,
    pub case_id: CaseId,
    pub revision: DeadlineRevision,
    pub definition: DeadlineDefinition,
    pub calculation: DeadlineCalculation,
    pub tracking: Option<DeadlineTrackingCapture>,
    pub responsible: DeadlineResponsibleSnapshot,
    pub attention: DeadlineAttention,
    pub status: DeadlineStatus,
    pub reason: Option<FactText>,
    pub receipt: DeadlineReceipt,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: DeadlineActorSnapshot,
}
/// Current authorization belongs to the store; this value captures its exact inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlinePreparation {
    pub case_id: CaseId,
    pub deadline_id: DeadlineId,
    pub administration: CurrentCaseAdministration,
    pub base: Option<DeadlineDetail>,
    pub resolved: Option<DeadlineResolvedInputs>,
    pub responsible: Option<DeadlineResponsibleSnapshot>,
}
