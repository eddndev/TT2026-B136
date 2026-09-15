use super::{CaseStageChange, StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot};
use crate::documents::{DocumentRecord, DocumentVersionRef};

/// Owned encrypted snapshot whose exact content and captured evidence were checked.
pub struct ValidatedStageSupport {
    record: DocumentRecord,
    format: StageDocumentFormat,
}

impl ValidatedStageSupport {
    pub fn record(&self) -> &DocumentRecord {
        &self.record
    }
    pub const fn format(&self) -> StageDocumentFormat {
        self.format
    }
    pub const fn policy(&self) -> StageFormatPolicy {
        StageFormatPolicy::PdfDocxV1
    }
    pub fn snapshot(&self) -> StageSupportSnapshot {
        StageSupportSnapshot {
            reference: DocumentVersionRef {
                id: self.record.id,
                version: self.record.version,
            },
            digest: self.record.digest,
            name: self.record.name.clone(),
            format: self.format,
            policy: self.policy(),
        }
    }
}

/// Only the application service can create this value after one validation batch.
pub struct PreparedCaseStageChange {
    change: CaseStageChange,
    supports: Vec<ValidatedStageSupport>,
}

impl PreparedCaseStageChange {
    pub fn change(&self) -> &CaseStageChange {
        &self.change
    }
    pub fn supports(&self) -> &[ValidatedStageSupport] {
        &self.supports
    }

    pub(super) fn new(
        change: CaseStageChange,
        records: Vec<DocumentRecord>,
        formats: Vec<StageDocumentFormat>,
    ) -> Self {
        Self {
            change,
            supports: records
                .into_iter()
                .zip(formats)
                .map(|(record, format)| ValidatedStageSupport { record, format })
                .collect(),
        }
    }
}
