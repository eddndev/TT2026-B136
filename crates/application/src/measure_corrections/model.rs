use crate::{
    case_stages::StageSupportSnapshot,
    identity::Principal,
    precautionary_hearings::{PrecautionaryContext, PrecautionaryContextExpectation},
    precautionary_measures::{
        MeasureCaptureAction, MeasureGroupRef, MeasureOriginIds, MeasureSourceProjection,
        MeasureSources,
    },
    typed_participants::SubjectSnapshot,
};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    hearings::HearingNote,
    precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef},
    precautionary_measures::{
        MeasureCorrectionOperationId, MeasureCorrectionValues, MeasureValues,
    },
    typed_participants::SubjectRevisionRef,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureAdministrativeAction {
    Correct(MeasureCorrectionValues),
    MarkEnteredInError,
    MarkEnteredInErrorAndReplace {
        replacement_id: MeasureId,
        subject: SubjectRevisionRef,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeReplacementMaterial {
    pub context: PrecautionaryContext,
    pub subject: SubjectSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeReplacementLink {
    pub entered_in_error: PrecautionaryMeasureRef,
    pub replacement: PrecautionaryMeasureRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeCommand {
    pub operation_id: MeasureCorrectionOperationId,
    pub target: PrecautionaryMeasureRef,
    pub context: PrecautionaryContextExpectation,
    pub reason: HearingNote,
    pub action: MeasureAdministrativeAction,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureJudicialRef {
    pub owner: MeasureGroupRef,
    pub reference: PrecautionaryMeasureRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureRecordRoot {
    Judicial(MeasureOriginIds),
    Administrative {
        operation_id: MeasureCorrectionOperationId,
        measure_id: MeasureId,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasureCaptureValidity {
    Valid,
    EnteredInError,
}
/// Supplied administrative projection; only complete reconstruction proves consistency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeResult {
    pub id: MeasureId,
    pub revision: MeasureRevision,
    pub previous: PrecautionaryMeasureRef,
    pub record_root: MeasureRecordRoot,
    pub judicial_origin: MeasureOriginIds,
    pub last_judicial: MeasureJudicialRef,
    pub last_action: MeasureCaptureAction,
    pub validity: MeasureCaptureValidity,
    pub values: MeasureValues,
    pub sources: MeasureSources,
    pub projection: MeasureSourceProjection,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeReview {
    pub case_id: CaseId,
    pub actor: Principal,
    pub command: MeasureAdministrativeCommand,
    pub context: PrecautionaryContext,
    pub support: StageSupportSnapshot,
    pub result: MeasureAdministrativeResult,
    pub replacement: Option<MeasureAdministrativeResult>,
    pub submission_digest: Sha256Digest,
    pub review_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeRecordCapture {
    pub case_id: CaseId,
    pub operation_id: MeasureCorrectionOperationId,
    pub result: MeasureAdministrativeResult,
    pub actor: Principal,
    pub context: PrecautionaryContext,
    pub support: StageSupportSnapshot,
    pub review_digest: Sha256Digest,
    pub recorded_at: OffsetDateTime,
    pub capture_digest: Sha256Digest,
}
/// Owns every administrative row without embedding an ancestor administrative receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeCapture {
    pub review: MeasureAdministrativeReview,
    pub records: Vec<MeasureAdministrativeRecordCapture>,
    pub replacement_link: Option<MeasureAdministrativeReplacementLink>,
    pub recorded_at: OffsetDateTime,
    pub capture_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativeOrigin {
    pub case_id: CaseId,
    pub operation_id: MeasureCorrectionOperationId,
    pub submission_digest: Sha256Digest,
    pub review_digest: Sha256Digest,
    pub capture_digest: Sha256Digest,
}
