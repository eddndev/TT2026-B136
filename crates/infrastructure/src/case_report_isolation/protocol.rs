use application::{case_reports::*, ApplicationError};
use domain::crypto::{DocumentHasher, Sha256Digest};

pub(super) const HEADER_LEN: usize = 95;
const MAGIC: &[u8; 5] = b"TTRP1";
#[derive(Clone, Copy)]
pub(super) struct Context {
    pub report_id: CaseReportId,
    pub snapshot_digest: Sha256Digest,
    pub format: CaseReportFormat,
}
fn unavailable() -> ApplicationError {
    CaseReportError::RenderUnavailable.into()
}
fn capacity() -> ApplicationError {
    CaseReportError::CapacityExceeded.into()
}
fn tag(format: CaseReportFormat) -> u8 {
    match format {
        CaseReportFormat::Pdf => 1,
        CaseReportFormat::Csv => 2,
    }
}
fn error_status(error: &ApplicationError) -> u8 {
    match error {
        ApplicationError::CaseReport(CaseReportError::CapacityExceeded) => 1,
        ApplicationError::CaseReport(CaseReportError::RenderUnavailable) => 3,
        ApplicationError::CaseReport(CaseReportError::StoredInconsistent(_)) => 4,
        _ => 2,
    }
}
/// The framing is fixed-width and versioned independently from capture JSON.
pub(super) fn encode(
    context: Context,
    result: Result<Vec<u8>, ApplicationError>,
) -> Result<Vec<u8>, ApplicationError> {
    let mut output = vec![0; HEADER_LEN];
    output[..5].copy_from_slice(MAGIC);
    output[6] = tag(context.format);
    match result {
        Err(error) => output[5] = error_status(&error),
        Ok(content) => {
            if content.is_empty() {
                return Err(CaseReportError::RenderFailed.into());
            }
            if content.len() > MAX_REPORT_ARTIFACT_BYTES {
                return Err(capacity());
            }
            output
                .try_reserve_exact(content.len())
                .map_err(|_| capacity())?;
            output[7..15].copy_from_slice(&(content.len() as u64).to_be_bytes());
            output[15..47].copy_from_slice(crate::RingSha256Hasher.hash_bytes(&content).as_bytes());
            output[47..63].copy_from_slice(context.report_id.as_uuid().as_bytes());
            output[63..95].copy_from_slice(context.snapshot_digest.as_bytes());
            output.extend_from_slice(&content);
        }
    }
    Ok(output)
}
pub(super) fn decode(bytes: &[u8], expected: Context) -> Result<Vec<u8>, ApplicationError> {
    if bytes.len() < HEADER_LEN || &bytes[..5] != MAGIC || bytes[6] != tag(expected.format) {
        return Err(unavailable());
    }
    if bytes[5] != 0 {
        if bytes.len() != HEADER_LEN || bytes[7..].iter().any(|byte| *byte != 0) {
            return Err(unavailable());
        }
        return Err(match bytes[5] {
            1 => capacity(),
            2 => CaseReportError::RenderFailed.into(),
            3 => unavailable(),
            4 => CaseReportError::StoredInconsistent(
                "report worker rejected an invalid capture".into(),
            )
            .into(),
            _ => unavailable(),
        });
    }
    let length = u64::from_be_bytes(bytes[7..15].try_into().map_err(|_| unavailable())?);
    if length > MAX_REPORT_ARTIFACT_BYTES as u64 {
        return Err(capacity());
    }
    if length == 0
        || bytes.len() != HEADER_LEN + length as usize
        || bytes[47..63] != expected.report_id.as_uuid().as_bytes()[..]
        || bytes[63..95] != expected.snapshot_digest.as_bytes()[..]
        || bytes[15..47]
            != crate::RingSha256Hasher
                .hash_bytes(&bytes[HEADER_LEN..])
                .as_bytes()[..]
    {
        return Err(unavailable());
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(length as usize)
        .map_err(|_| capacity())?;
    result.extend_from_slice(&bytes[HEADER_LEN..]);
    Ok(result)
}
