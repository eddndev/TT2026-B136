use domain::{
    cases::CaseId,
    crypto::{DocumentVersionRef, Sha256Digest},
};
use zeroize::Zeroizing;

/// Content validation does not establish signature, certificate or timestamp validity.
/// Plaintext is deliberately excluded from Debug and serialization.
pub struct DocumentContent {
    pub case_id: CaseId,
    pub reference: DocumentVersionRef,
    pub file_name: String,
    pub digest: Sha256Digest,
    pub bytes: Zeroizing<Vec<u8>>,
}
