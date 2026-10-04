use crate::{agenda_support::*, case_support, deadline_technical_support as technical};
use application::{
    agenda::*,
    deadline_currentness::evaluate_deadline_currentness,
    deadline_tracking::TrackingPolicy,
    deadlines::DeadlineOverview,
    hearings::{HearingModality, HearingStatus, HearingStatusFilter, HearingTime},
    ApplicationError,
};
use domain::{
    case_administration::CaseAdministrativeStatus,
    cases::CaseId,
    crypto::Sha256Digest,
    identity::{Role, UserId},
    procedural_resources::ResourceId,
    resource_activities::ResourceActivityId,
    resource_hearings::{ResourceHearingId, ResourceHearingKind, ResourceHearingRevision},
};
use std::sync::Arc;
use time::{Duration, OffsetDateTime, UtcOffset};
use uuid::Uuid;

mockall::mock! {
    Store {}
    impl AgendaStore for Store {
        fn list(&self, actor: UserId, query: AgendaQuery) -> Result<AgendaPage, ApplicationError>;
    }
}

fn resource_hearing(at: OffsetDateTime, id: Uuid) -> AgendaItem {
    let case_id = CaseId::from_uuid(Uuid::from_u128(1));
    AgendaItem::ResourceHearing {
        case: AgendaCaseSummary {
            case_id,
            title: "Authorized case".into(),
            reference: "CASE-1".into(),
            status: CaseAdministrativeStatus::Active,
        },
        hearing: Box::new(ResourceHearingAgendaOverview {
            case_id,
            resource_id: ResourceId::from_uuid(Uuid::from_u128(4)),
            id: ResourceHearingId::from_uuid(id),
            revision: ResourceHearingRevision::initial(),
            kind: ResourceHearingKind::AppealArguments,
            scheduled_at: HearingTime::new(at).unwrap(),
            modality: HearingModality::Videoconference,
            participant_count: 0,
            association_id: ResourceActivityId::from_uuid(Uuid::from_u128(5)),
            capture_digest: Sha256Digest::from_array([7; 32]),
        }),
    }
}

fn filtered(kind: AgendaKind, status: HearingStatusFilter) -> AgendaQuery {
    AgendaQuery::new(3, from(), until(), kind, status, None).unwrap()
}

#[test]
fn equal_instants_and_uuids_keep_three_distinct_families_in_order() {
    let base = technical::accepted(TrackingPolicy::Follow);
    let resolved = technical::heads(&base);
    let current = evaluate_deadline_currentness(
        technical::inputs::hasher().as_ref(),
        &base,
        Some(&resolved),
        checked_at(),
    )
    .unwrap();
    let deadline = DeadlineOverview::from(&current);
    let at = deadline.operational.due_at().unwrap();
    let id = deadline.id.as_uuid();
    let own = resource_hearing(at, id);
    let AgendaItem::ResourceHearing { case, .. } = &own else {
        unreachable!()
    };
    let due = AgendaItem::Deadline {
        case: AgendaCaseSummary {
            case_id: deadline.case_id,
            ..case.clone()
        },
        deadline: Box::new(deadline),
    };
    let rows = vec![AgendaItem::Hearing(hearing(at, id.as_u128())), due, own];
    let result = page(rows.clone());
    assert!(result.validate(&query(3)).is_ok());
    let keys: Vec<_> = rows.iter().map(|row| row.key().unwrap()).collect();
    assert_eq!(keys[2].kind(), AgendaItemKind::ResourceHearing);
    assert!(keys[0] < keys[1] && keys[1] < keys[2]);
    assert!(keys.iter().all(|key| key.id() == id && key.at() == at));
    let mut reversed = rows;
    reversed.swap(1, 2);
    assert!(page(reversed).validate(&query(3)).is_err());
}

#[test]
fn resource_projection_preserves_exact_parent_capture_and_local_offset_in_closed_case() {
    let at = from().to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap());
    for kind in [
        ResourceHearingKind::AppealArguments,
        ResourceHearingKind::WrittenRevocation,
    ] {
        for count in [0, 32] {
            let mut value = resource_hearing(at, Uuid::from_u128(9));
            let AgendaItem::ResourceHearing { case, hearing } = &mut value else {
                unreachable!()
            };
            case.status = CaseAdministrativeStatus::Closed;
            hearing.kind = kind;
            hearing.participant_count = count;
            let expected = hearing.clone();
            assert!(page(vec![value.clone()]).validate(&query(1)).is_ok());
            assert_eq!(value.key().unwrap().at().offset(), UtcOffset::UTC);
            let AgendaItem::ResourceHearing { hearing, .. } = value else {
                unreachable!()
            };
            assert_eq!(hearing, expected);
            assert_eq!(hearing.scheduled_at.value().offset(), at.offset());
        }
    }
}

#[test]
fn resource_projection_rejects_wrong_scope_invalid_metadata_count_and_revision() {
    for mutation in 0..5 {
        let mut value = resource_hearing(from(), Uuid::from_u128(9));
        let AgendaItem::ResourceHearing { case, hearing } = &mut value else {
            unreachable!()
        };
        match mutation {
            0 => case.case_id = CaseId::new(),
            1 => case.title = " padded".into(),
            2 => case.reference = "".into(),
            3 => hearing.participant_count = 33,
            _ => hearing.revision = ResourceHearingRevision::new(2).unwrap(),
        }
        assert!(value.key().is_err(), "mutation {mutation}");
        assert!(page(vec![value]).validate(&query(1)).is_err());
    }
}

#[test]
fn resource_family_filters_are_explicit_and_cancellation_does_not_apply_to_it() {
    let value = resource_hearing(from(), Uuid::from_u128(9));
    for status in [HearingStatusFilter::Scheduled, HearingStatusFilter::All] {
        for kind in [AgendaKind::All, AgendaKind::ResourceHearing] {
            assert!(page(vec![value.clone()])
                .validate(&filtered(kind, status))
                .is_ok());
        }
    }
    for kind in [AgendaKind::Hearing, AgendaKind::Deadline] {
        assert!(page(vec![value.clone()])
            .validate(&filtered(kind, HearingStatusFilter::Scheduled))
            .is_err());
    }
    assert!(matches!(
        AgendaQuery::new(
            1,
            from(),
            until(),
            AgendaKind::ResourceHearing,
            HearingStatusFilter::Cancelled,
            None
        ),
        Err(ApplicationError::InvalidInput(_))
    ));
    let cancelled = filtered(AgendaKind::All, HearingStatusFilter::Cancelled);
    assert!(page(vec![value]).validate(&cancelled).is_err());
    let mut ordinary = hearing(from(), 9);
    ordinary.status = HearingStatus::Cancelled;
    assert!(page(vec![AgendaItem::Hearing(ordinary)])
        .validate(&cancelled)
        .is_ok());
    assert!(page(vec![item(9)])
        .validate(&filtered(
            AgendaKind::ResourceHearing,
            HearingStatusFilter::All
        ))
        .is_err());
}

#[test]
fn resource_cursor_keeps_nanos_filter_binding_and_empty_page_continuation() {
    let at = from() + Duration::nanoseconds(123_456_789);
    let id = Uuid::from_u128(9);
    let cursor = AgendaCursor::new(at, AgendaItemKind::ResourceHearing, id).unwrap();
    assert_eq!(cursor.at().nanosecond(), 123_456_789);
    let q = AgendaQuery::new(
        1,
        from(),
        until(),
        AgendaKind::ResourceHearing,
        HearingStatusFilter::Scheduled,
        Some(cursor),
    )
    .unwrap();
    assert_eq!(q.after(), Some(cursor));
    assert!(page(vec![resource_hearing(from(), id)])
        .validate(&q)
        .is_err());
    let later = resource_hearing(from() + Duration::seconds(1), id);
    let next = later.key().unwrap();
    assert!(page(vec![later]).validate(&q).is_ok());
    let empty = AgendaPage {
        checked_at: checked_at(),
        items: vec![],
        complete: false,
        next_after: Some(next),
    };
    assert!(empty.validate(&q).is_ok());
    for kind in [AgendaKind::Hearing, AgendaKind::Deadline] {
        assert!(AgendaQuery::new(
            1,
            from(),
            until(),
            kind,
            HearingStatusFilter::Scheduled,
            Some(cursor)
        )
        .is_err());
    }
    assert!(AgendaQuery::new(
        1,
        from(),
        until(),
        AgendaKind::All,
        HearingStatusFilter::Cancelled,
        Some(cursor)
    )
    .is_err());
}

#[test]
fn resource_range_is_half_open_and_keyset_can_continue_after_another_family() {
    let id = Uuid::from_u128(9);
    let before = AgendaCursor::new(from(), AgendaItemKind::Deadline, id).unwrap();
    let q = AgendaQuery::new(
        2,
        from(),
        until(),
        AgendaKind::All,
        HearingStatusFilter::Scheduled,
        Some(before),
    )
    .unwrap();
    let own = resource_hearing(from(), id);
    assert!(page(vec![own.clone()]).validate(&q).is_ok());
    assert!(page(vec![own.clone(), own]).validate(&q).is_err());
    for at in [from() - Duration::seconds(1), until()] {
        assert!(page(vec![resource_hearing(at, id)])
            .validate(&query(1))
            .is_err());
    }
}

#[test]
fn resource_family_reauthenticates_staff_and_denies_clients_before_storage() {
    let q = filtered(AgendaKind::ResourceHearing, HearingStatusFilter::Scheduled);
    for role in [Role::Owner, Role::Litigator, Role::Paralegal, Role::Client] {
        let allowed = role != Role::Client;
        let (identity, actor) = case_support::identity(role, if allowed { 2 } else { 1 });
        let mut store = MockStore::new();
        if allowed {
            store
                .expect_list()
                .times(1)
                .withf(move |id, query| *id == actor.id && *query == q)
                .return_once(|_, _| Ok(page(vec![resource_hearing(from(), Uuid::from_u128(9))])));
        } else {
            store.expect_list().times(0);
        }
        let service = AgendaService::new(Arc::new(store), Arc::new(identity));
        let result = service.list("session", q);
        if allowed {
            assert_eq!(result.unwrap().items.len(), 1);
        } else {
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
        }
    }
}
