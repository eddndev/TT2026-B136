use super::*;
use application::{
    case_stages::{CaseStage, CaseStageRevision},
    cases::CaseRevision,
    deadline_currentness::{evaluate_deadline_currentness, DeadlineCurrent},
    deadline_profiles::{DeadlineCivilCutoff, DeadlineCompletionPolicy, DeadlineProfileDefinition},
    deadlines::*,
    hearings::*,
    resource_activities::*,
};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    procedural_facts::{FactLabel, FactText},
    procedural_resources::{ResourceId, ResourceRevision},
};
use time::{OffsetDateTime, Time, UtcOffset};
use uuid::Uuid;
#[path = "../../../application/tests/deadline_profile_catalog_support/values.rs"]
mod profiles;
pub fn digest() -> Sha256Digest {
    resources::digest()
}
pub fn case() -> CaseId {
    CaseId::from_uuid(CASE.parse().unwrap())
}
pub fn resource() -> ResourceId {
    ResourceId::from_uuid(RESOURCE.parse().unwrap())
}
pub fn association() -> ResourceActivityId {
    ResourceActivityId::from_uuid(ID.parse().unwrap())
}
pub fn now() -> OffsetDateTime {
    deadlines::records::instant() + time::Duration::days(1)
}
fn hearing(revision: u32) -> HearingDetail {
    let r = resources::detail(case(), &resources::command(resource(), 2));
    HearingDetail {
        snapshot: HearingSnapshot {
            case_id: case(),
            id: HearingId::from_uuid(HEARING.parse().unwrap()),
            revision: HearingRevision::new(revision).unwrap(),
            values: HearingValues::new(HearingValuesInput {
                kind: HearingKind::Initial,
                scheduled_at: HearingTime::new(now()).unwrap(),
                modality: HearingModality::InPerson,
                venue: HearingVenue::new("Court A").unwrap(),
                note: None,
                participants: vec![],
                conviction_basis: None,
            })
            .unwrap(),
            values_digest: digest(),
            status: HearingStatus::Scheduled,
            reason: (revision > 1).then(|| HearingNote::new("Rescheduled").unwrap()),
            receipt: HearingReceipt {
                operation_id: HearingOperationId::from_uuid(Uuid::nil()),
                action: if revision == 1 {
                    HearingAction::Schedule
                } else {
                    HearingAction::Replace
                },
                expected_revision: revision - 1,
                expected_context: Some(HearingContextExpectation {
                    case_revision: CaseRevision::FIRST,
                    stage_revision: CaseStageRevision::FIRST,
                }),
                submission_digest: digest(),
            },
            scheduling_context: HearingSchedulingContext {
                administration_revision: CaseRevision::FIRST,
                administration_digest: digest(),
                stage_revision: CaseStageRevision::FIRST,
                stage: CaseStage::Investigation,
                stage_digest: None,
            },
            recorded_administration_revision: CaseRevision::FIRST,
            recorded_administration_digest: digest(),
            recorded_at: now(),
            recorded_by: r.recorded_by,
        },
        participants: vec![],
        support: None,
    }
}
fn deadline() -> DeadlineDetail {
    let mut row = deadlines::records::fixture();
    let mut input = profiles::input(Some(case()));
    input.conditions[0].id = Uuid::nil();
    input.completion = DeadlineCompletionPolicy::CivilCutoff(
        DeadlineCivilCutoff::new(
            Time::from_hms(17, 0, 0).unwrap(),
            UtcOffset::UTC,
            "2026-01-01".parse().unwrap(),
            "2026-12-31".parse().unwrap(),
            FactLabel::new("Synthetic channel").unwrap(),
            Uuid::nil(),
        )
        .unwrap(),
    );
    row.calculation.profile.definition = DeadlineProfileDefinition::new(input).unwrap();
    let command = DeadlineHumanCommand::new(
        DeadlineCommand {
            operation_id: row.receipt.operation_id,
            deadline_id: row.id,
            change: DeadlineChange::Register {
                definition: row.definition.clone(),
            },
        },
        Some(deadlines::records::tracking_policies()),
    )
    .unwrap();
    deadlines::records::tracked_change(command, &row)
}
fn current_deadline() -> DeadlineCurrent {
    let base = deadline();
    let command = DeadlineHumanCommand::new(
        DeadlineCommand {
            operation_id: DeadlineOperationId::from_uuid(Uuid::from_u128(88)),
            deadline_id: base.id,
            change: DeadlineChange::Retire {
                expected_revision: base.revision,
                reason: FactText::new("Organizational retirement").unwrap(),
            },
        },
        None,
    )
    .unwrap();
    let head = deadlines::records::tracked_change(command, &base);
    evaluate_deadline_currentness(&deadlines::records::Hasher, &head, None, now()).unwrap()
}
pub fn command(kind: ResourceActivityKind, unlink: bool) -> ResourceActivityCommand {
    ResourceActivityCommand {
        operation_id: ResourceActivityOperationId::from_uuid(OP.parse().unwrap()),
        association_id: association(),
        expected_resource_revision: ResourceRevision::new(5).unwrap(),
        change: if unlink {
            ResourceActivityChange::Unlink {
                expected_revision: ResourceActivityRevision::initial(),
                reason: FactText::new("Organizational unlink").unwrap(),
            }
        } else {
            ResourceActivityChange::Link {
                selection: selection(kind),
            }
        },
    }
}
fn selection(kind: ResourceActivityKind) -> ResourceActivitySelection {
    let target = match kind {
        ResourceActivityKind::Hearing => ResourceActivityTarget::Hearing {
            id: HearingId::from_uuid(HEARING.parse().unwrap()),
            revision: HearingRevision::initial(),
            submission_digest: digest(),
        },
        ResourceActivityKind::Deadline => ResourceActivityTarget::Deadline {
            id: DeadlineId::from_uuid(Uuid::nil()),
            revision: DeadlineRevision::initial(),
            capture_digest: digest(),
        },
    };
    ResourceActivitySelection {
        resource: ResourceCaptureRef {
            id: resource(),
            revision: ResourceRevision::new(2).unwrap(),
            capture_digest: digest(),
        },
        act: None,
        target,
    }
}
pub fn detail(c: &ResourceActivityCommand, kind: ResourceActivityKind) -> ResourceActivityDetail {
    let source = resources::detail(case(), &resources::command(resource(), 2));
    let mut row = ResourceActivityDetail {
        case_id: case(),
        resource_id: resource(),
        id: c.association_id,
        revision: c.result_revision().unwrap(),
        selection: selection(kind),
        status: if c.action() == ResourceActivityAction::Link {
            ResourceActivityStatus::Linked
        } else {
            ResourceActivityStatus::Unlinked
        },
        sources: ResourceActivitySources {
            resource: source.clone(),
            act: None,
            target: match kind {
                ResourceActivityKind::Hearing => {
                    ResourceActivityTargetDetail::Hearing(Box::new(hearing(1)))
                }
                ResourceActivityKind::Deadline => {
                    ResourceActivityTargetDetail::Deadline(Box::new(deadline()))
                }
            },
        },
        reason: c.reason().cloned(),
        receipt: ResourceActivityReceipt {
            operation_id: c.operation_id,
            action: c.action(),
            expected_revision: c.expected_revision(),
            expected_resource_revision: c.expected_resource_revision,
            previous: (c.expected_revision() > 0).then(|| ResourceActivityRevisionRef {
                revision: ResourceActivityRevision::new(c.expected_revision()).unwrap(),
                capture_digest: digest(),
            }),
            submission_digest: digest(),
            capture_digest: digest(),
        },
        recorded_by: source.recorded_by,
        recorded_at: now(),
        recorded_administration: source.recorded_administration,
        recorded_resource_head: ResourceCaptureRef {
            id: resource(),
            revision: c.expected_resource_revision,
            capture_digest: digest(),
        },
    };
    if let ResourceActivityChange::Link { selection } = &c.change {
        row.selection = *selection;
        row.sources.act = selection.act.map(|_| super::act::source());
    }
    row
}
pub fn view(c: &ResourceActivityCommand, kind: ResourceActivityKind) -> ResourceActivityView {
    ResourceActivityView {
        association: detail(c, kind),
        checked_at: now(),
        current_target: match kind {
            ResourceActivityKind::Hearing => {
                ResourceActivityCurrentTarget::Hearing(Box::new(hearing(2)))
            }
            ResourceActivityKind::Deadline => {
                ResourceActivityCurrentTarget::Deadline(Box::new(current_deadline()))
            }
        },
    }
}
pub fn draft(c: ResourceActivityCommand, kind: ResourceActivityKind) -> ResourceActivityDraft {
    let row = detail(&c, kind);
    ResourceActivityDraft {
        case_id: row.case_id,
        resource_id: row.resource_id,
        command: c,
        result_revision: row.revision,
        selection: row.selection,
        status: row.status,
        sources: row.sources,
        previous: row.receipt.previous,
        recorded_by: row.recorded_by,
        observed_administration: row.recorded_administration,
        observed_resource_head: row.recorded_resource_head,
        submission_digest: digest(),
    }
}
