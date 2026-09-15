use application::{
    documents::{DocumentFormatBatch, DocumentFormatBatchValidator, StageDocumentFormat},
    ApplicationError,
};
use std::{path::PathBuf, time::Duration};

/// Executes one bounded native parser process for the entire support batch.
pub struct IsolatedDocumentFormatValidator {
    executable: PathBuf,
    library: PathBuf,
}

impl IsolatedDocumentFormatValidator {
    /// Both paths are deployment configuration and must identify existing files.
    pub fn new(executable: PathBuf, library: PathBuf) -> Result<Self, ApplicationError> {
        if !executable.is_absolute()
            || !library.is_absolute()
            || !executable.is_file()
            || !library.is_file()
        {
            return Err(ApplicationError::InvalidConfiguration(
                "document worker and native library require absolute existing paths".into(),
            ));
        }
        Ok(Self {
            executable,
            library,
        })
    }

    /// Checks native loading and a complete PDF parse in the bounded worker.
    pub fn check_configuration(&self) -> Result<(), ApplicationError> {
        let sample = include_bytes!("../../tests/fixtures/stage-support.pdf");
        self.run(&[sample], Duration::from_secs(10)).map(|_| ())
    }

    #[cfg(target_os = "linux")]
    fn run(
        &self,
        inputs: &[&[u8]],
        timeout: Duration,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        transport::run(&self.executable, &self.library, inputs, timeout)
    }

    #[cfg(not(target_os = "linux"))]
    fn run(
        &self,
        _inputs: &[&[u8]],
        _timeout: Duration,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        let _ = (&self.executable, &self.library);
        Err(ApplicationError::InvalidConfiguration(
            "isolated document format validation requires Linux".into(),
        ))
    }
}

impl DocumentFormatBatchValidator for IsolatedDocumentFormatValidator {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        let inputs = batch
            .inputs()
            .iter()
            .map(|input| input.bytes())
            .collect::<Vec<_>>();
        self.run(&inputs, Duration::from_secs(10))
    }
}

#[cfg(target_os = "linux")]
mod transport;

#[cfg(all(test, target_os = "linux"))]
mod tests;
