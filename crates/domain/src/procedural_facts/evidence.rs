use super::FactLabel;
use crate::{
    crypto::{DocumentVersionRef, Sha256Digest},
    DomainError,
};

/// Exact content selected for admission; a caller-supplied digest is an expectation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactSupportRef {
    reference: DocumentVersionRef,
    digest: Sha256Digest,
}
impl FactSupportRef {
    pub const fn new(reference: DocumentVersionRef, digest: Sha256Digest) -> Self {
        Self { reference, digest }
    }
    pub const fn reference(self) -> DocumentVersionRef {
        self.reference
    }
    pub const fn digest(self) -> Sha256Digest {
        self.digest
    }
}

/// One documentary assertion keeps its locator independently of admission deduplication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactEvidence {
    support: FactSupportRef,
    locator: FactLabel,
}
impl FactEvidence {
    pub const fn new(
        reference: DocumentVersionRef,
        digest: Sha256Digest,
        locator: FactLabel,
    ) -> Self {
        Self {
            support: FactSupportRef::new(reference, digest),
            locator,
        }
    }
    pub const fn reference(&self) -> DocumentVersionRef {
        self.support.reference()
    }
    pub const fn digest(&self) -> Sha256Digest {
        self.support.digest()
    }
    pub const fn locator(&self) -> &FactLabel {
        &self.locator
    }
    pub const fn support(&self) -> FactSupportRef {
        self.support
    }
}

pub(super) fn direct_supports(
    primary: Option<&FactEvidence>,
    relationship: Option<&FactEvidence>,
) -> Result<Vec<FactSupportRef>, DomainError> {
    let mut result: Vec<FactSupportRef> = Vec::with_capacity(2);
    for evidence in [primary, relationship].into_iter().flatten() {
        if let Some(previous) = result
            .iter()
            .find(|p| p.reference() == evidence.reference())
        {
            if previous.digest() != evidence.digest() {
                return Err(DomainError::InvalidProceduralFact("support_digest"));
            }
        } else {
            result.push(evidence.support());
        }
    }
    Ok(result)
}
