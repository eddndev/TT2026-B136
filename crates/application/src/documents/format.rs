//! Bounded, cryptographically validated plaintext loans for structural admission.

mod limits;
pub use limits::StageSupportReadLimits;

use super::validation::{validated_plaintext_with_ports, DocumentValidationPorts};
use super::{DocumentRecord, DocumentVersionRef};
use crate::ApplicationError;

/// Supported structural formats; detection belongs to the batch adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageDocumentFormat {
    Pdf,
    Docx,
}

impl StageDocumentFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Docx => "docx",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageFormatPolicy {
    PdfDocxV1,
}

impl StageFormatPolicy {
    pub const fn as_str(self) -> &'static str {
        "pdf_docx_v1"
    }
}

/// A borrowed validated document. Plaintext is intentionally absent from Debug.
pub struct DocumentFormatInput<'a> {
    reference: DocumentVersionRef,
    bytes: &'a [u8],
}

impl DocumentFormatInput<'_> {
    pub const fn reference(&self) -> DocumentVersionRef {
        self.reference
    }
    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }
}

/// A single bounded batch whose owning zeroizing buffers remain in the caller.
pub struct DocumentFormatBatch<'a> {
    inputs: Vec<DocumentFormatInput<'a>>,
}

impl DocumentFormatBatch<'_> {
    pub fn inputs(&self) -> &[DocumentFormatInput<'_>] {
        &self.inputs
    }
    pub fn total_bytes(&self) -> usize {
        self.inputs.iter().map(|input| input.bytes.len()).sum()
    }
}

/// Runs one structural validation budget for all inputs, returning formats in order.
/// Implementations must neither log plaintext nor treat format admission as legal proof.
pub trait DocumentFormatBatchValidator: Send + Sync {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError>;
}

pub(super) fn validate_support_batch(
    records: &[DocumentRecord],
    kek: &[u8],
    ports: &DocumentValidationPorts<'_>,
    limits: &StageSupportReadLimits,
    validator: &dyn DocumentFormatBatchValidator,
) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
    if records.is_empty() || records.len() > limits.max_documents() {
        return Err(ApplicationError::InvalidInput(
            "support batch must contain one or two documents".into(),
        ));
    }
    if records.len() == 2
        && records[0].id == records[1].id
        && records[0].version == records[1].version
    {
        return Err(ApplicationError::InvalidInput(
            "support batch contains duplicate document versions".into(),
        ));
    }
    let lengths = records
        .iter()
        .map(|record| limits.check_record(record))
        .collect::<Result<Vec<_>, _>>()?;
    let total = lengths
        .iter()
        .try_fold(0usize, |total, length| total.checked_add(*length))
        .ok_or(ApplicationError::StageSupportTooLarge)?;
    if total > limits.max_batch_plaintext_bytes() {
        return Err(ApplicationError::StageSupportTooLarge);
    }
    let mut plaintexts = Vec::with_capacity(records.len());
    for (record, expected_length) in records.iter().zip(lengths) {
        let plaintext = validated_plaintext_with_ports(record, kek, ports)?;
        if plaintext.len() != expected_length {
            return Err(ApplicationError::StoredDocumentInconsistent(
                "decrypted length differs from AES vault layout".into(),
            ));
        }
        plaintexts.push(plaintext);
    }
    let batch = DocumentFormatBatch {
        inputs: records
            .iter()
            .zip(&plaintexts)
            .map(|(record, plaintext)| DocumentFormatInput {
                reference: DocumentVersionRef {
                    id: record.id,
                    version: record.version,
                },
                bytes: plaintext,
            })
            .collect(),
    };
    let formats = validator.validate_batch(&batch)?;
    if formats.len() != records.len() {
        return Err(ApplicationError::Port(
            "format validator returned an invalid result count".into(),
        ));
    }
    Ok(formats)
}
