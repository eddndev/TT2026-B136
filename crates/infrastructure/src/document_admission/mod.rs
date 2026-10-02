//! Bounded admission of supported new-upload formats.

mod structure;

#[cfg(test)]
mod structure_tests;

#[cfg(target_os = "linux")]
mod process;
#[cfg(all(test, target_os = "linux"))]
mod process_tests;

mod media;
#[cfg(test)]
mod media_tests;

mod isolated;
mod worker;
pub use isolated::IsolatedDocumentUploadAdmission;
pub use worker::worker_entry;
