use crate::{
    case_stages::StageSupportSnapshot,
    documents::{
        DocumentFormatBatchValidator, DocumentProcessor, DocumentRecord, StageSupportReadLimits,
    },
    ApplicationError,
};
use domain::precautionary_measures::MeasureDecisionValues;

/// Admits the exact resolution support through bounded integrity and format checks.
/// Case association and current access remain the authorized store's responsibility.
pub fn admit_measure_decision_support(
    values: &MeasureDecisionValues,
    records: &[DocumentRecord],
    processor: &DocumentProcessor,
    limits: &StageSupportReadLimits,
    validator: &dyn DocumentFormatBatchValidator,
) -> Result<StageSupportSnapshot, ApplicationError> {
    crate::documents::admit_exact_stage_support(
        values.support(),
        records,
        processor,
        limits,
        validator,
    )
}
