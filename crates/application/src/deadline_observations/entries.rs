use crate::{
    deadline_inputs::DeadlineSourceDetail,
    deadline_profiles::{DeadlineProfileDetail, DeadlineProfileScope},
    deadline_reevaluation::{
        DependencyFamily, ObservationEntry, ObservationRole, ResolutionReference,
    },
    deadlines::evidence,
    judicial_calendars::JudicialCalendarDetail,
    procedural_facts::ProceduralFactSnapshot,
};
use domain::crypto::DocumentHasher;

fn bytes(family: DependencyFamily) -> Vec<u8> {
    let mut bytes = b"DLOE1".to_vec();
    bytes.push(family as u8);
    bytes
}

pub(super) fn profile(
    hasher: &dyn DocumentHasher,
    value: &DeadlineProfileDetail,
) -> ObservationEntry {
    let family = DependencyFamily::Profile;
    let mut bytes = bytes(family);
    evidence::profile(&mut bytes, value);
    ObservationEntry {
        role: ObservationRole::Profile,
        family,
        id: value.id.as_uuid(),
        revision: value.revision.get(),
        case_id: match value.definition.scope() {
            DeadlineProfileScope::Case(id) => Some(*id),
            DeadlineProfileScope::Global(_) => None,
        },
        hearing_id: None,
        parent_resolution: None,
        submission_digest: value.receipt.submission_digest,
        evidence_digest: hasher.hash_bytes(&bytes),
    }
}

pub(super) fn calendar(
    hasher: &dyn DocumentHasher,
    value: &JudicialCalendarDetail,
) -> ObservationEntry {
    let family = DependencyFamily::Calendar;
    let mut bytes = bytes(family);
    evidence::calendar(&mut bytes, value);
    ObservationEntry {
        role: ObservationRole::Calendar,
        family,
        id: value.id.as_uuid(),
        revision: value.revision.get(),
        case_id: None,
        hearing_id: None,
        parent_resolution: None,
        submission_digest: value.receipt.submission_digest,
        evidence_digest: hasher.hash_bytes(&bytes),
    }
}

pub(super) fn source(
    hasher: &dyn DocumentHasher,
    role: ObservationRole,
    value: &DeadlineSourceDetail,
) -> ObservationEntry {
    let (family, id, revision, case_id, hearing_id, parent_resolution, submission_digest) =
        match value {
            DeadlineSourceDetail::Fact(detail) => {
                let metadata = detail.snapshot.metadata();
                let (family, id, parent) = match &detail.snapshot {
                    ProceduralFactSnapshot::Resolution(value) => (
                        DependencyFamily::Resolution,
                        value.root.id().as_uuid(),
                        None,
                    ),
                    ProceduralFactSnapshot::Notification(value) => {
                        let exact = value.values.resolution();
                        (
                            DependencyFamily::Notification,
                            value.root.id().as_uuid(),
                            Some(ResolutionReference {
                                id: exact.id.as_uuid(),
                                revision: exact.revision.get(),
                            }),
                        )
                    }
                };
                (
                    family,
                    id,
                    metadata.revision.get(),
                    detail.snapshot.case_id(),
                    None,
                    parent,
                    metadata.receipt.submission_digest,
                )
            }
            DeadlineSourceDetail::HearingResult(detail) => {
                let value = &detail.snapshot;
                (
                    DependencyFamily::HearingResult,
                    value.id.as_uuid(),
                    value.revision.get(),
                    value.case_id,
                    Some(value.hearing_id.as_uuid()),
                    None,
                    value.receipt.submission_digest,
                )
            }
        };
    let mut bytes = bytes(family);
    // Preserve the existing inner source tag (1/2/3) after the DLOE1 family tag (0/1/2).
    evidence::source(&mut bytes, hasher, Some(value));
    ObservationEntry {
        role,
        family,
        id,
        revision,
        case_id: Some(case_id),
        hearing_id,
        parent_resolution,
        submission_digest,
        evidence_digest: hasher.hash_bytes(&bytes),
    }
}
