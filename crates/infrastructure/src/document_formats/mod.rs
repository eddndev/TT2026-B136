//! Native format admission used by the isolated document validation worker.

mod docx;
mod isolated;
mod pdf;
mod worker;

pub use isolated::IsolatedDocumentFormatValidator;
use pdf::PdfLibrary;
pub use worker::worker_entry;
