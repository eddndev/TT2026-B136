use super::*;
use crate::{
    documents::DocumentRecord, identity::Principal, measure_corrections::OwnedMeasureRecord,
    precautionary_hearings::PrecautionaryContext,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest};

/// Exact sources and ancestors; the store separately proves live access and heads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionRecordReady {
    pub context: PrecautionaryContext,
    pub support_record: DocumentRecord,
    pub anchor: Option<MeasureDecisionAnchorMaterial>,
    pub predecessors: Vec<OwnedMeasureRecord>,
    pub result_sources: Vec<MeasureResultSources>,
    pub record_history: MeasureDecisionRecordHistoryEvidence,
}

/// Original V2 group and its exact ancestor closure, excluding this owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionRecordStoredOperation {
    pub group: MeasureDecisionGroupCaptureV2,
    pub origin: MeasureGroupOrigin,
    pub record_history: MeasureDecisionRecordHistoryEvidence,
}

/// Retains the recorded family without upgrading or reframing original evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureDecisionRecordReceipt {
    V1(Box<MeasureDecisionStoredOperation>),
    V2(Box<MeasureDecisionRecordStoredOperation>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureDecisionRecordReview {
    V1(Box<MeasureDecisionReview>),
    V2(Box<MeasureDecisionReviewV2>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureDecisionRecordPreparation {
    Ready(Box<MeasureDecisionRecordReady>),
    Replay(Box<MeasureDecisionRecordReceipt>),
}

impl MeasureDecisionRecordReceipt {
    pub fn case_id(&self) -> CaseId {
        self.origin().case_id
    }
    pub fn actor(&self) -> &Principal {
        match self {
            Self::V1(value) => &value.group.review.actor,
            Self::V2(value) => &value.group.review.actor,
        }
    }
    pub fn command(&self) -> &MeasureDecisionCommand {
        match self {
            Self::V1(value) => &value.group.review.command,
            Self::V2(value) => &value.group.review.command,
        }
    }
    pub fn submission_digest(&self) -> Sha256Digest {
        self.origin().submission_digest
    }
    pub fn review_digest(&self) -> Sha256Digest {
        self.origin().review_digest
    }
    pub fn recorded_at(&self) -> OffsetDateTime {
        match self {
            Self::V1(value) => value.group.recorded_at,
            Self::V2(value) => value.group.recorded_at,
        }
    }
    pub fn origin(&self) -> &MeasureGroupOrigin {
        match self {
            Self::V1(value) => &value.origin,
            Self::V2(value) => &value.origin,
        }
    }
    pub(super) fn into_review(self) -> MeasureDecisionRecordReview {
        match self {
            Self::V1(value) => MeasureDecisionRecordReview::V1(Box::new(value.group.review)),
            Self::V2(value) => MeasureDecisionRecordReview::V2(Box::new(value.group.review)),
        }
    }
}

impl MeasureDecisionRecordReview {
    pub fn case_id(&self) -> CaseId {
        match self {
            Self::V1(value) => value.case_id,
            Self::V2(value) => value.case_id,
        }
    }
    pub fn actor(&self) -> &Principal {
        match self {
            Self::V1(value) => &value.actor,
            Self::V2(value) => &value.actor,
        }
    }
    pub fn command(&self) -> &MeasureDecisionCommand {
        match self {
            Self::V1(value) => &value.command,
            Self::V2(value) => &value.command,
        }
    }
    pub fn submission_digest(&self) -> Sha256Digest {
        match self {
            Self::V1(value) => value.submission_digest,
            Self::V2(value) => value.submission_digest,
        }
    }
    pub fn review_digest(&self) -> Sha256Digest {
        match self {
            Self::V1(value) => value.review_digest,
            Self::V2(value) => value.review_digest,
        }
    }
}
