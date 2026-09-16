use super::*;
use crate::hearings::HearingDetail;
impl From<&HearingDetail> for HearingResultAnchorSnapshot {
    fn from(detail: &HearingDetail) -> Self {
        let s = &detail.snapshot;
        Self {
            reference: HearingResultAnchor {
                hearing_id: s.id,
                revision: s.revision,
                values_digest: s.values_digest,
                submission_digest: s.receipt.submission_digest,
            },
            status: s.status,
            kind: s.values.kind(),
            scheduled_at: s.values.scheduled_at(),
            scheduling_context: s.scheduling_context,
        }
    }
}
impl From<&HearingResultSnapshot> for HearingResultContinuationSnapshot {
    fn from(s: &HearingResultSnapshot) -> Self {
        Self {
            reference: HearingResultContinuation {
                hearing_id: s.hearing_id,
                result_id: s.id,
                revision: s.revision,
                values_digest: s.values_digest,
                submission_digest: s.receipt.submission_digest,
            },
            status: s.status,
        }
    }
}
impl From<&HearingResultSnapshot> for HearingResultHistoryEntry {
    fn from(s: &HearingResultSnapshot) -> Self {
        Self {
            case_id: s.case_id,
            hearing_id: s.hearing_id,
            id: s.id,
            revision: s.revision,
            values_digest: s.values_digest,
            status: s.status,
            reason: s.reason.clone(),
            receipt: s.receipt.clone(),
            anchor: s.anchor,
            continuation: s.continuation,
            recorded_administration_revision: s.recorded_administration_revision,
            recorded_administration_digest: s.recorded_administration_digest,
            recorded_at: s.recorded_at,
            recorded_by: s.recorded_by.clone(),
        }
    }
}
