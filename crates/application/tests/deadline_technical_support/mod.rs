#![allow(dead_code)]
pub use crate::deadline_support::evaluation::{self, inputs};
use crate::{deadline_observation_support as observed, deadline_support};
use application::{
    deadline_inputs::{DeadlineCalendarRef, DeadlineSourceDetail},
    deadline_profiles::{DeadlineProfileDefinition, DeadlineProfileScope},
    deadline_reevaluation::*,
    deadline_technical::*,
    deadline_tracking::*,
    deadlines::*,
    judicial_calendars::JudicialCalendarDetail,
    procedural_facts::{FactDetail, ProceduralFactSnapshot},
};
use domain::{
    deadline_arithmetic::{ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy},
    deadline_profiles::DeadlineRuleTemplate,
    deadline_triggers::{TriggerField, TriggerRequirement},
    judicial_calendars::JudicialCalendarClassification,
};
use uuid::Uuid;

pub fn policies(source: TrackingPolicy) -> TrackingPolicies {
    TrackingPolicies {
        profile: TrackingPolicy::Follow,
        source,
        calendar: TrackingPolicy::Undetermined,
    }
}
pub fn legacy() -> DeadlineDetail {
    let (command, preparation) = deadline_support::fixture();
    deadline_support::detail(&deadline_support::prepare(command, preparation).unwrap())
}
pub fn human(
    command: DeadlineCommand,
    preparation: DeadlinePreparation,
    policies: Option<TrackingPolicies>,
    parent: Option<&FactDetail>,
) -> DeadlineDetail {
    let prepared = prepare_tracked_deadline_change(
        inputs::hasher().as_ref(),
        DeadlineActorSnapshot::User {
            id: inputs::actor(),
            email: "operator@example.test".into(),
        },
        inputs::case_id(),
        command,
        preparation,
        policies,
        parent,
    )
    .unwrap();
    let mut value = deadline_support::detail(&prepared);
    value.tracking = prepared.tracking().cloned();
    value.receipt = prepared.receipt();
    value.recorded_by = prepared.tracked_author().unwrap().clone();
    deadline_receipt_matches(inputs::hasher().as_ref(), &value).unwrap();
    value
}
pub fn accepted(source: TrackingPolicy) -> DeadlineDetail {
    let (command, preparation) = deadline_support::fixture();
    human(command, preparation, Some(policies(source)), None)
}
pub fn attention(base: &DeadlineDetail) -> DeadlineDetail {
    let (command, preparation) = deadline_support::followup(
        base,
        DeadlineChange::SetAttention {
            expected_revision: base.revision,
            attention: deadline_support::attention(),
            reason: evaluation::text("Operator declares a filing"),
        },
    );
    human(command, preparation, None, None)
}
pub fn retired(base: &DeadlineDetail) -> DeadlineDetail {
    let (command, preparation) = deadline_support::followup(
        base,
        DeadlineChange::Retire {
            expected_revision: base.revision,
            reason: evaluation::text("Operator retires the deadline"),
        },
    );
    human(command, preparation, None, None)
}
pub fn calendar_base(source: TrackingPolicy, calendar: TrackingPolicy) -> DeadlineDetail {
    let (mut command, mut preparation) = deadline_support::fixture();
    let resolved = preparation.resolved.as_mut().unwrap();
    let selected = inputs::calendar(1, false, JudicialCalendarClassification::Countable);
    let mut profile = observed::profile_input(&resolved.profile);
    profile.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::Days {
        quantity: evaluation::n(2),
        inclusion: DayInclusion::OnAnchor,
        basis: DayBasis::CalendarCountable,
        final_day: FinalDayPolicy::Preserve,
    });
    profile.examples[0].calendar = Some(selected.values.clone());
    resolved.profile.definition = DeadlineProfileDefinition::new(profile).unwrap();
    observed::resign_profile(&mut resolved.profile);
    resolved.profile_head = resolved.profile.clone();
    resolved.material.calendar = Some(selected.clone());
    resolved.material.calendar_head = Some(selected.clone());
    let DeadlineChange::Register { definition } = &mut command.change else {
        unreachable!()
    };
    definition.input.calendar = Some(DeadlineCalendarRef {
        id: selected.id,
        revision: selected.revision,
    });
    human(
        command,
        preparation,
        Some(TrackingPolicies {
            calendar,
            ..policies(source)
        }),
        None,
    )
}
pub fn notification_base(policy: TrackingPolicy) -> DeadlineDetail {
    let (mut command, mut preparation) = deadline_support::fixture();
    let resolved = preparation.resolved.as_mut().unwrap();
    let mut profile = observed::profile_input(&resolved.profile);
    profile.trigger = TriggerRequirement::SourceField(TriggerField::NotificationPracticedAt);
    resolved.profile.definition = DeadlineProfileDefinition::new(profile).unwrap();
    observed::resign_profile(&mut resolved.profile);
    resolved.profile_head = resolved.profile.clone();
    let source =
        DeadlineSourceDetail::Fact(Box::new(inputs::notification(1, false, 1, "2026-01-06")));
    let DeadlineChange::Register { definition } = &mut command.change else {
        unreachable!()
    };
    definition.input.selection = inputs::request(&source).trigger;
    resolved.material = inputs::material(source);
    let parent = inputs::resolution(1, false, "2026-01-01");
    human(command, preparation, Some(policies(policy)), Some(&parent))
}
pub fn heads(base: &DeadlineDetail) -> DeadlineReevaluationInputs {
    DeadlineReevaluationInputs {
        profile_head: base.calculation.profile.clone(),
        material: base.calculation.material.clone(),
        notification_parent_head: None,
    }
}
pub fn source_heads(
    base: &DeadlineDetail,
    revision: u32,
    withdrawn: bool,
) -> DeadlineReevaluationInputs {
    let mut value = heads(base);
    value.material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
        revision,
        withdrawn,
        "2026-01-10",
    ))));
    value
}
pub fn calendar_heads(
    base: &DeadlineDetail,
    revision: u32,
    retired: bool,
) -> DeadlineReevaluationInputs {
    let mut value = heads(base);
    value.material.calendar_head = Some(inputs::calendar(
        revision,
        retired,
        JudicialCalendarClassification::Unresolved,
    ));
    value
}
pub fn bootstrap() -> DeadlineReevaluationCommand {
    command(
        TechnicalCause::LegacyBootstrap {
            job_id: Uuid::from_u128(900),
            policy_version: 1,
        },
        900,
    )
}
fn command(cause: TechnicalCause, sequence: u64) -> DeadlineReevaluationCommand {
    DeadlineReevaluationCommand {
        operation_id: DeadlineOperationId::from_uuid(Uuid::from_u128(
            10_000 + u128::from(sequence),
        )),
        cause,
    }
}
pub fn event_command(event: SourceEventReference) -> DeadlineReevaluationCommand {
    command(
        TechnicalCause::SourceEvent {
            job_id: Uuid::from_u128(20_000 + u128::from(event.sequence)),
            event,
        },
        event.sequence,
    )
}
pub fn fact_event(fact: &FactDetail, sequence: u64) -> SourceEventReference {
    let (family, id) = match &fact.snapshot {
        ProceduralFactSnapshot::Resolution(value) => {
            (DependencyFamily::Resolution, value.root.id().as_uuid())
        }
        ProceduralFactSnapshot::Notification(value) => {
            (DependencyFamily::Notification, value.root.id().as_uuid())
        }
    };
    SourceEventReference {
        sequence,
        family,
        source_id: id,
        revision: fact.snapshot.metadata().revision.get(),
        case_id: Some(fact.snapshot.case_id()),
        hearing_id: None,
        operation_id: fact.snapshot.metadata().receipt.operation_id.as_uuid(),
    }
}
pub fn source_event(heads: &DeadlineReevaluationInputs, sequence: u64) -> SourceEventReference {
    fact_event(observed::fact(&heads.material.source_head), sequence)
}
pub fn calendar_event(value: &JudicialCalendarDetail, sequence: u64) -> SourceEventReference {
    SourceEventReference {
        sequence,
        family: DependencyFamily::Calendar,
        source_id: value.id.as_uuid(),
        revision: value.revision.get(),
        case_id: None,
        hearing_id: None,
        operation_id: value.receipt.operation_id.as_uuid(),
    }
}
pub fn profile_event(heads: &DeadlineReevaluationInputs, sequence: u64) -> SourceEventReference {
    let profile = &heads.profile_head;
    SourceEventReference {
        sequence,
        family: DependencyFamily::Profile,
        source_id: profile.id.as_uuid(),
        revision: profile.revision.get(),
        case_id: match profile.definition.scope() {
            DeadlineProfileScope::Global(_) => None,
            DeadlineProfileScope::Case(id) => Some(*id),
        },
        hearing_id: None,
        operation_id: profile.receipt.operation_id.as_uuid(),
    }
}
pub fn prepare(
    base: &DeadlineDetail,
    command: DeadlineReevaluationCommand,
    heads: DeadlineReevaluationInputs,
) -> Result<DeadlineReevaluationOutcome, application::ApplicationError> {
    prepare_technical_deadline_change(inputs::hasher().as_ref(), base, command, heads)
}
pub fn revision(
    base: &DeadlineDetail,
    command: DeadlineReevaluationCommand,
    heads: DeadlineReevaluationInputs,
) -> DeadlineDetail {
    let expected_profile = heads.profile_head.clone();
    let DeadlineReevaluationOutcome::Revision(prepared) = prepare(base, command, heads).unwrap()
    else {
        panic!("a technical revision is required")
    };
    assert_eq!(prepared.base(), base);
    assert_eq!(prepared.inputs().profile_head, expected_profile);
    assert_eq!(prepared.receipt().expected_revision, base.revision.get());
    let at = crate::case_support::instant() + time::Duration::hours(1);
    let next = prepared.record(at);
    assert_eq!(next.recorded_at, at);
    assert_eq!(prepared.definition(), &next.definition);
    assert_eq!(prepared.calculation(), &next.calculation);
    assert_eq!(prepared.tracking(), next.tracking.as_ref().unwrap());
    assert_eq!(
        prepared.receipt().submission_digest,
        next.receipt.submission_digest
    );
    deadline_receipt_matches(inputs::hasher().as_ref(), &next).unwrap();
    deadline_successor_matches(inputs::hasher().as_ref(), base, &next).unwrap();
    next
}
pub fn observation(value: &DeadlineDetail, role: ObservationRole) -> &ObservationEntry {
    observed::entry(&value.tracking.as_ref().unwrap().observations, role)
}
pub fn has_reason(
    value: &DeadlineDetail,
    dependency: TrackingDependency,
    reason: TrackingReviewReason,
) -> bool {
    value
        .tracking
        .as_ref()
        .unwrap()
        .review
        .reasons()
        .contains(&TrackingReviewRequirement { dependency, reason })
}
