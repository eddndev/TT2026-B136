use super::{capture_validation::invalid, PrecautionaryContext, PrecautionaryHearingReview};
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
pub(crate) struct SourceInventory<'a> {
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
        self.context(&review.scheduling_context)?;
        self.context(&review.observed_context)?;
        for source in &review.sources.participants {
            self.participant(source)?;
        }
        self.support(&review.sources.support)
    }

    pub(crate) fn context(
        &mut self,
        context: &'a PrecautionaryContext,
    ) -> Result<(), ApplicationError> {
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
        Ok(())
    }

    pub(crate) fn participant(
        &mut self,
        source: &'a ParticipantDetail,
    ) -> Result<(), ApplicationError> {
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
            self.subject(subject)?;
        }
        Ok(())
    }

    pub(crate) fn subject(&mut self, source: &'a SubjectSnapshot) -> Result<(), ApplicationError> {
        retain(
            &mut self.subjects,
            (
                *source.case_id.as_uuid().as_bytes(),
                *source.id.as_uuid().as_bytes(),
                source.revision.get(),
            ),
            source,
        )
    }

    pub(crate) fn support(
        &mut self,
        source: &'a StageSupportSnapshot,
    ) -> Result<(), ApplicationError> {
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
