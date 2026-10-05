use super::MeasureDecisionRecordReceipt;
use domain::{cases::CaseId, precautionary_measures::MeasureDecisionId};

/// Original immutable decisions with their actual receipt families and ancestors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionRecordPage {
    pub case_id: CaseId,
    pub items: Vec<MeasureDecisionRecordReceipt>,
    pub has_more: bool,
    pub next_after_id: Option<MeasureDecisionId>,
}
