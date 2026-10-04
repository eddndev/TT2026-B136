use super::*;
use domain::{
    crypto::Sha256Digest,
    procedural_resources::ResourceId,
    resource_activities::ResourceActivityId,
    resource_hearings::{ResourceHearingId, ResourceHearingKind, ResourceHearingRevision},
};
use serde_json::json;

mod cursor_tests;
mod projection_tests;

fn resource_hearing() -> AgendaItem {
    AgendaItem::ResourceHearing {
        case: AgendaCaseSummary {
            case_id: CaseId::from_uuid(Uuid::from_u128(1)),
            title: "Authorized resource case".into(),
            reference: "RESOURCE-1".into(),
            status: CaseAdministrativeStatus::Closed,
        },
        hearing: Box::new(ResourceHearingAgendaOverview {
            case_id: CaseId::from_uuid(Uuid::from_u128(1)),
            resource_id: ResourceId::from_uuid(Uuid::from_u128(3)),
            id: ResourceHearingId::from_uuid(Uuid::from_u128(2)),
            revision: ResourceHearingRevision::initial(),
            kind: ResourceHearingKind::AppealArguments,
            scheduled_at: HearingTime::new(
                at(1767225601).to_offset(time::UtcOffset::from_hms(-6, 0, 0).unwrap()),
            )
            .unwrap(),
            modality: HearingModality::InPerson,
            participant_count: 2,
            association_id: ResourceActivityId::from_uuid(Uuid::from_u128(4)),
            capture_digest: Sha256Digest::from_array([7; 32]),
        }),
    }
}

fn own_page() -> AgendaPage {
    AgendaPage {
        items: vec![resource_hearing()],
        ..page()
    }
}

fn empty_page() -> AgendaPage {
    AgendaPage {
        items: Vec::new(),
        ..page()
    }
}

fn timed_deadline() -> application::deadlines::DeadlineDetail {
    use super::deadline_http_support::records;
    use application::{deadline_profiles::*, deadlines::*};
    use domain::procedural_facts::FactLabel;
    let mut base = records::fixture();
    let mut profile = records::profiles::input(Some(base.case_id));
    profile.conditions[0].id = Uuid::nil();
    profile.completion = DeadlineCompletionPolicy::CivilCutoff(
        DeadlineCivilCutoff::new(
            time::Time::from_hms(17, 30, 0).unwrap(),
            time::UtcOffset::from_hms(-6, 0, 0).unwrap(),
            "2026-01-01".parse().unwrap(),
            "2026-12-31".parse().unwrap(),
            FactLabel::new("Synthetic agenda channel").unwrap(),
            Uuid::nil(),
        )
        .unwrap(),
    );
    base.calculation.profile.definition = DeadlineProfileDefinition::new(profile).unwrap();
    let command = DeadlineHumanCommand::new(
        DeadlineCommand {
            operation_id: base.receipt.operation_id,
            deadline_id: base.id,
            change: DeadlineChange::Register {
                definition: base.definition.clone(),
            },
        },
        Some(records::tracking_policies()),
    )
    .unwrap();
    records::tracked_change(command, &base)
}

fn mixed_page() -> AgendaPage {
    use super::deadline_http_support::records;
    use application::{
        deadline_currentness::evaluate_deadline_currentness,
        deadline_technical::DeadlineReevaluationInputs, deadlines::DeadlineOverview,
    };
    let base = timed_deadline();
    let inputs = DeadlineReevaluationInputs {
        profile_head: base.calculation.profile.clone(),
        material: base.calculation.material.clone(),
        notification_parent_head: None,
    };
    let current =
        evaluate_deadline_currentness(&records::Hasher, &base, Some(&inputs), records::instant())
            .unwrap();
    let deadline = DeadlineOverview::from(&current);
    let date = deadline.operational.due_at().unwrap();
    assert_eq!(
        date.format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
        "2026-01-06T23:30:00Z"
    );
    let id = deadline.id.as_uuid();
    let mut ordinary = hearing();
    let AgendaItem::Hearing(value) = &mut ordinary else {
        unreachable!()
    };
    value.id = HearingId::from_uuid(id);
    value.scheduled_at = HearingTime::new(date).unwrap();
    let mut own = resource_hearing();
    let AgendaItem::ResourceHearing { hearing, .. } = &mut own else {
        unreachable!()
    };
    hearing.id = ResourceHearingId::from_uuid(id);
    hearing.scheduled_at = HearingTime::new(date).unwrap();
    let due = AgendaItem::Deadline {
        case: AgendaCaseSummary {
            case_id: deadline.case_id,
            title: "Authorized deadline case".into(),
            reference: "DUE-1".into(),
            status: CaseAdministrativeStatus::Active,
        },
        deadline: Box::new(deadline),
    };
    AgendaPage {
        checked_at: records::instant(),
        items: vec![ordinary, due, own],
        complete: true,
        next_after: None,
    }
}
