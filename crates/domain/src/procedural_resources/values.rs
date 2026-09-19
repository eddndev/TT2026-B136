use super::{ResourceKind, ResourceMode};
use crate::{
    procedural_facts::{
        FactDeclaration, FactEvidence, FactLabel, FactParticipantRef, FactResolutionRef,
        FactSupportRef, FactText,
    },
    procedural_time::DeclaredProceduralTime,
    DomainError,
};
use std::collections::HashSet;

/// Technical bound on one captured list, not a limit on procedural standing.
pub const MAX_RESOURCE_APPELLANTS: usize = 32;

/// Captured declaration and optional directory selection, never an access account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceAppellant {
    name: FactLabel,
    role: FactDeclaration<FactLabel>,
    participant: Option<FactParticipantRef>,
}
impl ResourceAppellant {
    pub const fn new(
        name: FactLabel,
        role: FactDeclaration<FactLabel>,
        participant: Option<FactParticipantRef>,
    ) -> Self {
        Self {
            name,
            role,
            participant,
        }
    }
    pub const fn name(&self) -> &FactLabel {
        &self.name
    }
    pub const fn role(&self) -> &FactDeclaration<FactLabel> {
        &self.role
    }
    pub const fn participant(&self) -> Option<FactParticipantRef> {
        self.participant
    }
}

#[derive(Debug, Clone)]
pub struct ResourceValuesInput {
    pub kind: ResourceKind,
    pub mode: FactDeclaration<ResourceMode>,
    pub title: FactLabel,
    pub resolution: FactResolutionRef,
    pub resolution_evidence: FactEvidence,
    pub resolution_reference: FactDeclaration<FactLabel>,
    pub issuing_authority: FactDeclaration<FactLabel>,
    pub receiving_authority: Option<FactDeclaration<FactLabel>>,
    pub resolution_at: DeclaredProceduralTime,
    pub notification_at: Option<DeclaredProceduralTime>,
    pub challenged_part: FactText,
    pub grounds: FactText,
    pub appellants: Vec<ResourceAppellant>,
}

/// Organizational capture, independent of acts and active or archived status.
/// References and supplied digests are expectations until verified by a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceValues {
    kind: ResourceKind,
    mode: FactDeclaration<ResourceMode>,
    title: FactLabel,
    resolution: FactResolutionRef,
    resolution_evidence: FactEvidence,
    resolution_reference: FactDeclaration<FactLabel>,
    issuing_authority: FactDeclaration<FactLabel>,
    receiving_authority: Option<FactDeclaration<FactLabel>>,
    resolution_at: DeclaredProceduralTime,
    notification_at: Option<DeclaredProceduralTime>,
    challenged_part: FactText,
    grounds: FactText,
    appellants: Vec<ResourceAppellant>,
}
impl ResourceValues {
    pub fn new(input: ResourceValuesInput) -> Result<Self, DomainError> {
        let mut selected = HashSet::new();
        if input.appellants.is_empty()
            || input.appellants.len() > MAX_RESOURCE_APPELLANTS
            || input
                .appellants
                .iter()
                .filter_map(ResourceAppellant::participant)
                .any(|reference| !selected.insert(reference.id))
        {
            return Err(DomainError::InvalidProceduralResource("appellants"));
        }
        Ok(Self {
            kind: input.kind,
            mode: input.mode,
            title: input.title,
            resolution: input.resolution,
            resolution_evidence: input.resolution_evidence,
            resolution_reference: input.resolution_reference,
            issuing_authority: input.issuing_authority,
            receiving_authority: input.receiving_authority,
            resolution_at: input.resolution_at,
            notification_at: input.notification_at,
            challenged_part: input.challenged_part,
            grounds: input.grounds,
            appellants: input.appellants,
        })
    }
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }
    pub const fn mode(&self) -> &FactDeclaration<ResourceMode> {
        &self.mode
    }
    pub const fn title(&self) -> &FactLabel {
        &self.title
    }
    pub const fn resolution(&self) -> FactResolutionRef {
        self.resolution
    }
    pub const fn resolution_evidence(&self) -> &FactEvidence {
        &self.resolution_evidence
    }
    pub const fn resolution_reference(&self) -> &FactDeclaration<FactLabel> {
        &self.resolution_reference
    }
    pub const fn issuing_authority(&self) -> &FactDeclaration<FactLabel> {
        &self.issuing_authority
    }
    pub const fn receiving_authority(&self) -> Option<&FactDeclaration<FactLabel>> {
        self.receiving_authority.as_ref()
    }
    pub const fn resolution_at(&self) -> DeclaredProceduralTime {
        self.resolution_at
    }
    pub const fn notification_at(&self) -> Option<DeclaredProceduralTime> {
        self.notification_at
    }
    pub const fn challenged_part(&self) -> &FactText {
        &self.challenged_part
    }
    pub const fn grounds(&self) -> &FactText {
        &self.grounds
    }
    pub fn appellants(&self) -> &[ResourceAppellant] {
        &self.appellants
    }
    pub fn direct_supports(&self) -> Vec<FactSupportRef> {
        vec![self.resolution_evidence.support()]
    }
}
