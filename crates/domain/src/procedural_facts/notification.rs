use super::{
    evidence::direct_supports, FactDeclaration, FactLabel, FactPerson, FactProvenance,
    FactRepresentation, FactResolutionRef, FactSupportRef, FactText, NotificationCharacter,
    NotificationContext, NotificationMedium, NotificationOutcome,
};
use crate::{procedural_time::DeclaredProceduralTime, DomainError};

/// An expressly stated effect refers to the primary provenance, without deriving an effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactStatedEffect {
    pub at: DeclaredProceduralTime,
    pub statement: FactText,
    pub locator: FactLabel,
}

/// Construction input; NotificationValues additionally rejects conflicting support digests.
#[derive(Debug, Clone)]
pub struct NotificationValuesInput {
    pub resolution: FactResolutionRef,
    pub character: FactDeclaration<NotificationCharacter>,
    pub medium: FactDeclaration<NotificationMedium>,
    pub context: FactDeclaration<NotificationContext>,
    pub outcome: FactDeclaration<NotificationOutcome>,
    pub subtype: Option<FactLabel>,
    pub practiced_at: DeclaredProceduralTime,
    pub received_at: Option<DeclaredProceduralTime>,
    pub stated_effect: Option<FactStatedEffect>,
    pub intended_recipient: FactDeclaration<FactPerson>,
    pub actual_receiver: FactDeclaration<FactPerson>,
    pub representation: FactRepresentation,
    pub summary: FactText,
    pub provenance: FactProvenance,
}

/// One declared practice, with independent person functions and temporal purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationValues {
    resolution: FactResolutionRef,
    character: FactDeclaration<NotificationCharacter>,
    medium: FactDeclaration<NotificationMedium>,
    context: FactDeclaration<NotificationContext>,
    outcome: FactDeclaration<NotificationOutcome>,
    subtype: Option<FactLabel>,
    practiced_at: DeclaredProceduralTime,
    received_at: Option<DeclaredProceduralTime>,
    stated_effect: Option<FactStatedEffect>,
    intended_recipient: FactDeclaration<FactPerson>,
    actual_receiver: FactDeclaration<FactPerson>,
    representation: FactRepresentation,
    summary: FactText,
    provenance: FactProvenance,
    direct_supports: Vec<FactSupportRef>,
}
impl NotificationValues {
    pub fn new(input: NotificationValuesInput) -> Result<Self, DomainError> {
        let supports = direct_supports(
            input.provenance.support(),
            input
                .representation
                .provenance()
                .and_then(FactProvenance::support),
        )?;
        Ok(Self {
            resolution: input.resolution,
            character: input.character,
            medium: input.medium,
            context: input.context,
            outcome: input.outcome,
            subtype: input.subtype,
            practiced_at: input.practiced_at,
            received_at: input.received_at,
            stated_effect: input.stated_effect,
            intended_recipient: input.intended_recipient,
            actual_receiver: input.actual_receiver,
            representation: input.representation,
            summary: input.summary,
            provenance: input.provenance,
            direct_supports: supports,
        })
    }
    pub const fn resolution(&self) -> FactResolutionRef {
        self.resolution
    }
    pub const fn character(&self) -> &FactDeclaration<NotificationCharacter> {
        &self.character
    }
    pub const fn medium(&self) -> &FactDeclaration<NotificationMedium> {
        &self.medium
    }
    pub const fn context(&self) -> &FactDeclaration<NotificationContext> {
        &self.context
    }
    pub const fn outcome(&self) -> &FactDeclaration<NotificationOutcome> {
        &self.outcome
    }
    pub const fn subtype(&self) -> Option<&FactLabel> {
        self.subtype.as_ref()
    }
    pub const fn practiced_at(&self) -> DeclaredProceduralTime {
        self.practiced_at
    }
    pub const fn received_at(&self) -> Option<DeclaredProceduralTime> {
        self.received_at
    }
    pub const fn stated_effect(&self) -> Option<&FactStatedEffect> {
        self.stated_effect.as_ref()
    }
    pub const fn intended_recipient(&self) -> &FactDeclaration<FactPerson> {
        &self.intended_recipient
    }
    pub const fn actual_receiver(&self) -> &FactDeclaration<FactPerson> {
        &self.actual_receiver
    }
    pub const fn representation(&self) -> &FactRepresentation {
        &self.representation
    }
    pub const fn summary(&self) -> &FactText {
        &self.summary
    }
    pub const fn provenance(&self) -> &FactProvenance {
        &self.provenance
    }
    pub fn direct_supports(&self) -> Vec<FactSupportRef> {
        self.direct_supports.clone()
    }
}
