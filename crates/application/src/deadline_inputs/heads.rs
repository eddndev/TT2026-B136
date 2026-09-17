use super::{DeadlineCalendarRef, DeadlineInputMaterial, DeadlineSourceDetail};
use crate::procedural_facts::ProceduralFactSnapshot;
use domain::{
    deadline_triggers::TriggerSourceRef,
    procedural_facts::{FactHearingRef, FactResolutionRef},
};

/// Exact revisions observed as heads, separate from the selected historical sources.
/// Capturing references does not establish integrity, authorization or currentness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineInputHeads {
    pub source: Option<TriggerSourceRef>,
    pub calendar: Option<DeadlineCalendarRef>,
}
impl DeadlineInputHeads {
    pub fn capture(material: &DeadlineInputMaterial) -> Self {
        Self {
            source: material.source_head.as_ref().map(source_reference),
            calendar: material
                .calendar_head
                .as_ref()
                .map(|calendar| DeadlineCalendarRef {
                    id: calendar.id,
                    revision: calendar.revision,
                }),
        }
    }
}
fn source_reference(source: &DeadlineSourceDetail) -> TriggerSourceRef {
    match source {
        DeadlineSourceDetail::Fact(detail) => match &detail.snapshot {
            ProceduralFactSnapshot::Resolution(snapshot) => {
                TriggerSourceRef::Resolution(FactResolutionRef {
                    id: snapshot.root.id(),
                    revision: snapshot.metadata.revision,
                })
            }
            ProceduralFactSnapshot::Notification(snapshot) => TriggerSourceRef::Notification {
                id: snapshot.root.id(),
                revision: snapshot.metadata.revision,
                resolution: snapshot.values.resolution(),
            },
        },
        DeadlineSourceDetail::HearingResult(detail) => {
            TriggerSourceRef::HearingResult(FactHearingRef {
                hearing_id: detail.snapshot.hearing_id,
                result_id: detail.snapshot.id,
                revision: detail.snapshot.revision,
                // A head observation does not select any agreement in that revision.
                agreement_id: None,
            })
        }
    }
}
