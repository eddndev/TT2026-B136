use super::{
    DocumentFormatBatchValidator, DocumentProcessor, DocumentRecord, StageFormatPolicy,
    StageSupportReadLimits,
};
use crate::{case_stages::StageSupportSnapshot, ApplicationError};
use domain::hearings::HearingSupportRef;

pub(crate) fn admit_exact_stage_support(
    selected: HearingSupportRef,
    records: &[DocumentRecord],
    processor: &DocumentProcessor,
    limits: &StageSupportReadLimits,
    validator: &dyn DocumentFormatBatchValidator,
) -> Result<StageSupportSnapshot, ApplicationError> {
    let [record] = records else {
        return Err(ApplicationError::InvalidInput(
            "exact selection requires exactly one support".into(),
        ));
    };
    if record.id != selected.reference().id
        || record.version != selected.reference().version
        || record.digest != selected.digest()
    {
        return Err(ApplicationError::InvalidInput(
            "document support differs from exact selection".into(),
        ));
    }
    let formats = processor.validate_support_batch(records, limits, validator)?;
    let [format] = formats.as_slice() else {
        return Err(ApplicationError::Port(
            "document admission returned an invalid result count".into(),
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
