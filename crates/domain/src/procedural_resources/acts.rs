use super::{ResourceActKind, ResourceMode};
use crate::{
    procedural_facts::{FactDeclaration, FactEvidence, FactLabel, FactSupportRef, FactText},
    procedural_time::DeclaredProceduralTime,
    DomainError,
};

/// Technical batch limit for exact supports; it is not a legal evidentiary rule.
pub const MAX_RESOURCE_ACT_EVIDENCE: usize = 2;

#[derive(Debug, Clone)]
pub struct ResourceActValuesInput {
    pub kind: ResourceActKind,
    pub mode: FactDeclaration<ResourceMode>,
    pub occurred_at: DeclaredProceduralTime,
    pub authority: FactDeclaration<FactLabel>,
    pub statement: FactText,
    pub evidence: Vec<FactEvidence>,
}

/// A supported declaration does not certify presentation, admissibility or finality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceActValues {
    kind: ResourceActKind,
    mode: FactDeclaration<ResourceMode>,
    occurred_at: DeclaredProceduralTime,
    authority: FactDeclaration<FactLabel>,
    statement: FactText,
    evidence: Vec<FactEvidence>,
    supports: Vec<FactSupportRef>,
}
impl ResourceActValues {
    pub fn new(input: ResourceActValuesInput) -> Result<Self, DomainError> {
        if input.evidence.is_empty() || input.evidence.len() > MAX_RESOURCE_ACT_EVIDENCE {
            return Err(DomainError::InvalidProceduralResource("act_evidence"));
        }
        let mut supports: Vec<FactSupportRef> = Vec::new();
        for evidence in &input.evidence {
            if let Some(previous) = supports
                .iter()
                .find(|value| value.reference() == evidence.reference())
            {
                if previous.digest() != evidence.digest() {
                    return Err(DomainError::InvalidProceduralResource("support_digest"));
                }
            } else {
                supports.push(evidence.support());
            }
        }
        Ok(Self {
            kind: input.kind,
            mode: input.mode,
            occurred_at: input.occurred_at,
            authority: input.authority,
            statement: input.statement,
            evidence: input.evidence,
            supports,
        })
    }
    pub const fn kind(&self) -> ResourceActKind {
        self.kind
    }
    pub const fn mode(&self) -> &FactDeclaration<ResourceMode> {
        &self.mode
    }
    pub const fn occurred_at(&self) -> DeclaredProceduralTime {
        self.occurred_at
    }
    pub const fn authority(&self) -> &FactDeclaration<FactLabel> {
        &self.authority
    }
    pub const fn statement(&self) -> &FactText {
        &self.statement
    }
    pub fn evidence(&self) -> &[FactEvidence] {
        &self.evidence
    }
    pub fn direct_supports(&self) -> Vec<FactSupportRef> {
        self.supports.clone()
    }
}
