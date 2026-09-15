use crate::crypto::{DocumentVersionRef, Sha256Digest};

/// The exact content requested as support; resolution never selects a later version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageSupportRef {
    reference: DocumentVersionRef,
    digest: Sha256Digest,
}

impl StageSupportRef {
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
