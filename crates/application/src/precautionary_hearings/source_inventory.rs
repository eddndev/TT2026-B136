use super::{capture_validation::invalid, PrecautionaryHearingReview};
use crate::{
    case_stages::{CaseStageEntry, StageSupportSnapshot},
    cases::CaseAdministrationSnapshot,
    typed_participants::{ParticipantDetail, SubjectSnapshot},
    ApplicationError,
};
use std::collections::{btree_map::Entry, BTreeMap};
type SourceId = [u8; 16];

/// Repeated immutable identities must retain all original values and provenance.
#[derive(Default)]
pub(super) struct SourceInventory<'a> {
    administrations: BTreeMap<(SourceId, u32), &'a CaseAdministrationSnapshot>,
    stages: BTreeMap<(SourceId, u32), &'a CaseStageEntry>,
    participants: BTreeMap<(SourceId, SourceId, u32), &'a ParticipantDetail>,
    subjects: BTreeMap<(SourceId, SourceId, u32), &'a SubjectSnapshot>,
    documents: BTreeMap<(SourceId, u32), &'a StageSupportSnapshot>,
}

impl<'a> SourceInventory<'a> {
    pub(super) fn add(
        &mut self,
        review: &'a PrecautionaryHearingReview,
    ) -> Result<(), ApplicationError> {
        for context in [&review.scheduling_context, &review.observed_context] {
            let material = context.material();
            for source in [&material.administration, &material.stage_administration] {
                retain(
                    &mut self.administrations,
                    (*source.case_id.as_uuid().as_bytes(), source.revision.get()),
                    source,
                )?;
            }
            let stage = &material.stage;
            retain(
                &mut self.stages,
                (
                    *stage.case_id().as_uuid().as_bytes(),
                    stage.stage_revision().get(),
                ),
                stage,
            )?;
            if let CaseStageEntry::Changed(source) = stage {
                for support in &source.supports {
                    self.support(support)?;
                }
            }
        }
        for source in &review.sources.participants {
            retain(
                &mut self.participants,
                (
                    *source.case_id().as_uuid().as_bytes(),
                    *source.id().as_uuid().as_bytes(),
                    source.revision_number().get(),
                ),
                source,
            )?;
            if let Some(subject) = &source.bound_subject {
                retain(
                    &mut self.subjects,
                    (
                        *subject.case_id.as_uuid().as_bytes(),
                        *subject.id.as_uuid().as_bytes(),
                        subject.revision.get(),
                    ),
                    subject,
                )?;
            }
        }
        self.support(&review.sources.support)
    }

    fn support(&mut self, source: &'a StageSupportSnapshot) -> Result<(), ApplicationError> {
        retain(
            &mut self.documents,
            (
                *source.reference.id.as_uuid().as_bytes(),
                source.reference.version.get(),
            ),
            source,
        )
    }
}

fn retain<'a, K: Ord, V: PartialEq>(
    inventory: &mut BTreeMap<K, &'a V>,
    key: K,
    value: &'a V,
) -> Result<(), ApplicationError> {
    match inventory.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(value);
        }
        Entry::Occupied(entry) if *entry.get() != value => {
            return Err(invalid(
                "immutable source identity has contradictory material",
            ));
        }
        Entry::Occupied(_) => {}
    }
    Ok(())
}
