use super::PrecautionaryHearingRecordStoredOperation;
use domain::{cases::CaseId, precautionary_hearings::PrecautionaryHearingId};

/// Each item retains its exact head, original prefix and full mixed owner proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingRecordPage {
    pub case_id: CaseId,
    pub items: Vec<PrecautionaryHearingRecordStoredOperation>,
    pub has_more: bool,
    pub next_after_id: Option<PrecautionaryHearingId>,
}
