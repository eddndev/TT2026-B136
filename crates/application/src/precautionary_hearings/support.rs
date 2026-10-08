use crate::{
    case_stages::StageSupportSnapshot,
    documents::{
        DocumentFormatBatchValidator, DocumentProcessor, DocumentRecord, StageSupportReadLimits,
    },
    ApplicationError,
};
use domain::precautionary_hearings::PrecautionaryHearingValues;

/// Admits the exact scheduling document through bounded cryptographic and format checks.
/// The authorized store must separately establish and recheck its case association.
pub fn admit_precautionary_support(
    values: &PrecautionaryHearingValues,
    records: &[DocumentRecord],
    processor: &DocumentProcessor,
    limits: &StageSupportReadLimits,
    validator: &dyn DocumentFormatBatchValidator,
) -> Result<StageSupportSnapshot, ApplicationError> {
    crate::documents::admit_exact_stage_support(
        values.scheduling_basis().support(),
        records,
        processor,
        limits,
        validator,
    )
}
