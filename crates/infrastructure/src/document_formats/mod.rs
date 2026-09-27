//! Native format admission used by the isolated document validation worker.

mod docx;
mod isolated;
mod pdf;
mod worker;

pub use isolated::IsolatedDocumentFormatValidator;
use pdf::PdfLibrary;
pub use worker::worker_entry;

/// Reuses the strict support parsers inside the general isolated worker.
pub(crate) fn validate_upload(
    bytes: &[u8],
    library: &std::path::Path,
) -> Result<(), application::ApplicationError> {
    if bytes.starts_with(b"%PDF-") {
        PdfLibrary::open(library)?.validate(bytes)
    } else if bytes.starts_with(b"PK\x03\x04") {
        docx::validate_docx(bytes, &mut docx::DocxBudget::standard())
    } else {
        Err(application::ApplicationError::StageSupportFormatRejected)
    }
}
