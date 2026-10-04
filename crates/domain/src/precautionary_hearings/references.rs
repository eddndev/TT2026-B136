use crate::crypto::Sha256Digest;
use crate::hearings::{HearingNote, HearingSupportRef};

use super::{MeasureId, MeasureRevision};

/// Exact captured measure selected for review, without substituting its current head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrecautionaryMeasureRef {
    id: MeasureId,
    revision: MeasureRevision,
    digest: Sha256Digest,
}

impl PrecautionaryMeasureRef {
    pub const fn new(id: MeasureId, revision: MeasureRevision, digest: Sha256Digest) -> Self {
        Self {
            id,
            revision,
            digest,
        }
    }

    pub const fn id(self) -> MeasureId {
        self.id
    }
    pub const fn revision(self) -> MeasureRevision {
        self.revision
    }
    pub const fn digest(self) -> Sha256Digest {
        self.digest
    }
}

/// Operator's scheduling declaration and exact source; not proof of judicial admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingSchedulingBasis {
    statement: HearingNote,
    support: HearingSupportRef,
    locator: HearingNote,
}

impl PrecautionaryHearingSchedulingBasis {
    pub const fn new(
        statement: HearingNote,
        support: HearingSupportRef,
        locator: HearingNote,
    ) -> Self {
        Self {
            statement,
            support,
            locator,
        }
    }

    pub const fn statement(&self) -> &HearingNote {
        &self.statement
    }
    pub const fn support(&self) -> HearingSupportRef {
        self.support
    }
    pub const fn locator(&self) -> &HearingNote {
        &self.locator
    }
}
