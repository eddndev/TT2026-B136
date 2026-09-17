use super::{
    FactDeclaration, FactLabel, FactProvenance, FactRevision, FactSupportRef, FactText,
    ResolutionClass, ResolutionId,
};
use crate::procedural_time::DeclaredProceduralTime;

/// Exact historical selection; source digests and status require repository resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactResolutionRef {
    pub id: ResolutionId,
    pub revision: FactRevision,
}

#[derive(Debug, Clone)]
pub struct ResolutionValuesInput {
    pub class: FactDeclaration<ResolutionClass>,
    pub subtype: Option<FactLabel>,
    pub issuer: FactDeclaration<FactLabel>,
    pub issued_at: DeclaredProceduralTime,
    pub summary: FactText,
    pub provenance: FactProvenance,
}

/// Immutable declaration; capture time and root identity are separate from these values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionValues {
    class: FactDeclaration<ResolutionClass>,
    subtype: Option<FactLabel>,
    issuer: FactDeclaration<FactLabel>,
    issued_at: DeclaredProceduralTime,
    summary: FactText,
    provenance: FactProvenance,
}
impl ResolutionValues {
    pub fn new(input: ResolutionValuesInput) -> Self {
        Self {
            class: input.class,
            subtype: input.subtype,
            issuer: input.issuer,
            issued_at: input.issued_at,
            summary: input.summary,
            provenance: input.provenance,
        }
    }
    pub const fn class(&self) -> &FactDeclaration<ResolutionClass> {
        &self.class
    }
    pub const fn subtype(&self) -> Option<&FactLabel> {
        self.subtype.as_ref()
    }
    pub const fn issuer(&self) -> &FactDeclaration<FactLabel> {
        &self.issuer
    }
    pub const fn issued_at(&self) -> DeclaredProceduralTime {
        self.issued_at
    }
    pub const fn summary(&self) -> &FactText {
        &self.summary
    }
    pub const fn provenance(&self) -> &FactProvenance {
        &self.provenance
    }
    pub fn direct_supports(&self) -> Vec<FactSupportRef> {
        self.provenance
            .support()
            .map(|v| v.support())
            .into_iter()
            .collect()
    }
}
