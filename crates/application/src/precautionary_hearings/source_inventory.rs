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
    commitments: super::context_commitments::ContextCommitments,
    hearing_operations: BTreeMap<(SourceId, SourceId), &'a PrecautionaryHearingReview>,
    hearing_reviews: BTreeMap<(SourceId, SourceId, u32), &'a PrecautionaryHearingReview>,
    projections: BTreeMap<
        (SourceId, SourceId, u32),
        (
            &'a crate::typed_participants::ParticipantOverview,
            domain::crypto::Sha256Digest,
        ),
    >,
    administrations: BTreeMap<(SourceId, u32), &'a CaseAdministrationSnapshot>,
    stages: BTreeMap<(SourceId, u32), &'a CaseStageEntry>,
    participants: BTreeMap<(SourceId, SourceId, u32), &'a ParticipantDetail>,
    subject_digests: BTreeMap<(SourceId, SourceId, u32), &'a domain::crypto::Sha256Digest>,
    subjects: BTreeMap<(SourceId, SourceId, u32), &'a SubjectSnapshot>,
    ordinary_hearings: BTreeMap<(SourceId, SourceId, u32), &'a crate::hearings::HearingDetail>,
    precautionary_hearings:
        BTreeMap<(SourceId, SourceId, u32), &'a super::PrecautionaryHearingCapture>,
    documents: BTreeMap<(SourceId, u32), &'a StageSupportSnapshot>,
}

impl<'a> SourceInventory<'a> {
    pub(crate) fn anchor(
        &mut self,
        anchor: &'a crate::precautionary_measures::MeasureDecisionAnchorMaterial,
    ) -> Result<(), ApplicationError> {
        use crate::precautionary_measures::MeasureDecisionAnchorMaterial as Anchor;
        match anchor {
            Anchor::Initial(detail) => {
                self.commitments.ordinary(detail)?;
                let s = &detail.snapshot;
                retain(
                    &mut self.ordinary_hearings,
                    (
                        *s.case_id.as_uuid().as_bytes(),
                        *s.id.as_uuid().as_bytes(),
                        s.revision.get(),
                    ),
                    detail,
                )?;
                for participant in &detail.participants {
                    self.projection(&participant.overview, participant.values_digest)?;
                }
                if let Some(support) = &detail.support {
                    self.support(support)?;
                }
            }
            Anchor::Precautionary(capture) => self.capture(capture)?,
        }
        Ok(())
    }

    pub(crate) fn capture(
        &mut self,
        capture: &'a super::PrecautionaryHearingCapture,
    ) -> Result<(), ApplicationError> {
        let r = &capture.review;
        retain(
            &mut self.precautionary_hearings,
            (
                *r.case_id.as_uuid().as_bytes(),
                *r.command.hearing_id.as_uuid().as_bytes(),
                r.result_revision.get(),
            ),
            capture,
        )?;
        self.add(r)
    }

    pub(crate) fn projection(
        &mut self,
        overview: &'a crate::typed_participants::ParticipantOverview,
        digest: domain::crypto::Sha256Digest,
    ) -> Result<(), ApplicationError> {
        let key = (
            *overview.case_id.as_uuid().as_bytes(),
            *overview.id.as_uuid().as_bytes(),
            overview.revision.get(),
        );
        if let Some(subject) = &overview.subject {
            retain(
                &mut self.subject_digests,
                (
                    *overview.case_id.as_uuid().as_bytes(),
                    *subject.id.as_uuid().as_bytes(),
                    subject.revision.get(),
                ),
                &subject.values_digest,
            )?;
        }
        let value = (overview, digest);
        match self.projections.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(value);
            }
            Entry::Occupied(entry) if *entry.get() != value => {
                return Err(invalid(
                    "immutable participant projection contradicts supplied evidence",
                ));
            }
            Entry::Occupied(_) => {}
        }
        Ok(())
    }

    pub(crate) fn add(
        &mut self,
        review: &'a PrecautionaryHearingReview,
    ) -> Result<(), ApplicationError> {
        retain(
            &mut self.hearing_operations,
            (
                *review.case_id.as_uuid().as_bytes(),
                *review.command.operation_id.as_uuid().as_bytes(),
            ),
            review,
        )?;
        retain(
            &mut self.hearing_reviews,
            (
                *review.case_id.as_uuid().as_bytes(),
                *review.command.hearing_id.as_uuid().as_bytes(),
                review.result_revision.get(),
            ),
            review,
        )?;
        for participant in &review.participants {
            self.projection(&participant.overview, participant.snapshot.values_digest)?;
        }
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
        self.commitments.context(context)?;
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
            &mut self.subject_digests,
            (
                *source.case_id.as_uuid().as_bytes(),
                *source.id.as_uuid().as_bytes(),
                source.revision.get(),
            ),
            &source.values_digest,
        )?;
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
