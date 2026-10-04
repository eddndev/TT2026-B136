use crate::{
    case_stages::StageSupportSnapshot,
    documents::{
        DocumentFormatBatchValidator, DocumentProcessor, DocumentRecord, StageFormatPolicy,
        StageSupportReadLimits,
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
    let [record] = records else {
        return Err(ApplicationError::InvalidInput(
            "precautionary scheduling requires exactly one support".into(),
        ));
    };
    let selected = values.scheduling_basis().support();
    if record.id != selected.reference().id
        || record.version != selected.reference().version
        || record.digest != selected.digest()
    {
        return Err(ApplicationError::InvalidInput(
            "precautionary support differs from exact selection".into(),
        ));
    }
    let formats = processor.validate_support_batch(records, limits, validator)?;
    let [format] = formats.as_slice() else {
        return Err(ApplicationError::Port(
            "precautionary admission returned an invalid result count".into(),
        ));
    };
    Ok(StageSupportSnapshot {
        reference: selected.reference(),
        digest: selected.digest(),
        name: record.name.clone(),
        format: *format,
        policy: StageFormatPolicy::PdfDocxV1,
    })
}
