//! Bounded exports over one capture; see docs/adr/0060-durable-authorized-case-reports.md.
mod bounds;
mod csv;
mod embed;
mod font;
mod layout;
mod pdf;

use application::{case_reports::*, ApplicationError};
use domain::crypto::DocumentHasher;

/// Offline renderer with the exact bundled, licensed fonts and no host lookup.
pub struct BoundedCaseReportRenderer {
    regular: Box<[u8]>,
    bold: Box<[u8]>,
}

impl BoundedCaseReportRenderer {
    /// Validates pinned bytes before parsing; arbitrary fonts are not accepted.
    pub fn new(regular: &[u8], bold: &[u8]) -> Result<Self, ApplicationError> {
        const REGULAR_SHA256: &str =
            "b85c38ecea8a7cfb39c24e395a4007474fa5a4fc864f6ee33309eb4948d232d5";
        const BOLD_SHA256: &str =
            "c976e4b1b99edc88775377fcc21692ca4bfa46b6d6ca6522bfda505b28ff9d6a";
        if regular.len() != 569208 || bold.len() != 575740 {
            return Err(unavailable());
        }
        let hasher = crate::RingSha256Hasher;
        if hasher.hash_bytes(regular).to_hex() != REGULAR_SHA256
            || hasher.hash_bytes(bold).to_hex() != BOLD_SHA256
        {
            return Err(unavailable());
        }
        font::Font::new(regular)?;
        font::Font::new(bold)?;
        Ok(Self {
            regular: regular.into(),
            bold: bold.into(),
        })
    }

    /// Uses only assets shipped with the application binary.
    pub fn bundled() -> Result<Self, ApplicationError> {
        Self::new(
            include_bytes!("../../assets/case-reports/NotoSans-Regular.ttf"),
            include_bytes!("../../assets/case-reports/NotoSans-Bold.ttf"),
        )
    }
}

impl CaseReportRenderer for BoundedCaseReportRenderer {
    fn render(
        &self,
        snapshot: &CaseReportSnapshot,
        format: CaseReportFormat,
    ) -> Result<Vec<u8>, ApplicationError> {
        bounds::capture(snapshot)?;
        match format {
            CaseReportFormat::Csv => csv::render(snapshot),
            CaseReportFormat::Pdf => pdf::render(snapshot, &self.regular, &self.bold),
        }
    }
}

fn capacity() -> ApplicationError {
    CaseReportError::CapacityExceeded.into()
}
fn failed() -> ApplicationError {
    CaseReportError::RenderFailed.into()
}
fn unavailable() -> ApplicationError {
    CaseReportError::RenderUnavailable.into()
}
