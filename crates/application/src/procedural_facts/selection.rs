use super::{
    FactDeclaration, FactHearingRef, FactParticipantRef, FactPerson, FactProvenance,
    FactRepresentation, FactResolutionRef, FactSupportRef, ProceduralFactValues,
};

/// Exact immediate references only; selection does not resolve or admit any source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactSourceSelection {
    resolution: Option<FactResolutionRef>,
    participants: Vec<FactParticipantRef>,
    hearing_results: Vec<FactHearingRef>,
    direct_supports: Vec<FactSupportRef>,
}
impl FactSourceSelection {
    /// One exact resolution selection for consumers that share fact verification.
    pub fn select_resolution(reference: FactResolutionRef) -> Self {
        Self {
            resolution: Some(reference),
            participants: vec![],
            hearing_results: vec![],
            direct_supports: vec![],
        }
    }
    /// One exact participant selection; the existing per-fact limits are unchanged.
    pub fn select_participant(reference: FactParticipantRef) -> Self {
        Self {
            resolution: None,
            participants: vec![reference],
            hearing_results: vec![],
            direct_supports: vec![],
        }
    }

    pub fn from_values(values: &ProceduralFactValues) -> Self {
        let mut selected = Self {
            resolution: None,
            participants: Vec::new(),
            hearing_results: Vec::new(),
            direct_supports: Vec::new(),
        };
        match values {
            ProceduralFactValues::Resolution(values) => {
                selected.provenance(values.provenance());
                selected.direct_supports = values.direct_supports();
            }
            ProceduralFactValues::Notification(values) => {
                selected.resolution = Some(values.resolution());
                for declaration in [values.intended_recipient(), values.actual_receiver()] {
                    if let FactDeclaration::Known(person) = declaration {
                        selected.person(person);
                    }
                }
                selected.provenance(values.provenance());
                if let FactRepresentation::Declared {
                    represented,
                    representative,
                    provenance,
                    ..
                } = values.representation()
                {
                    selected.person(represented);
                    selected.person(representative);
                    selected.provenance(provenance);
                }
                selected.direct_supports = values.direct_supports();
            }
        }
        selected.order_and_deduplicate();
        selected
    }
    pub const fn resolution(&self) -> Option<FactResolutionRef> {
        self.resolution
    }
    pub fn participants(&self) -> &[FactParticipantRef] {
        &self.participants
    }
    pub fn hearing_results(&self) -> &[FactHearingRef] {
        &self.hearing_results
    }
    pub fn direct_supports(&self) -> &[FactSupportRef] {
        &self.direct_supports
    }
    fn person(&mut self, person: &FactPerson) {
        if let FactPerson::Participant(reference) = person {
            self.participants.push(*reference);
        }
    }
    fn provenance(&mut self, provenance: &FactProvenance) {
        if let Some(reference) = provenance.hearing_reference() {
            self.hearing_results.push(reference);
        }
    }
    fn order_and_deduplicate(&mut self) {
        self.participants
            .sort_by_key(|reference| (reference.id.as_uuid(), reference.revision.get()));
        self.participants.dedup();
        self.hearing_results.sort_by_key(|reference| {
            (
                reference.hearing_id.as_uuid(),
                reference.result_id.as_uuid(),
                reference.revision.get(),
                reference.agreement_id.map(|id| id.as_uuid()),
            )
        });
        self.hearing_results.dedup();
        self.direct_supports.sort_by_key(|support| {
            let reference = support.reference();
            (reference.id.as_uuid(), reference.version.get())
        });
        self.direct_supports.dedup();
    }
}
