use crate::{agenda_support::*, deadline_technical_support as technical};
use application::{
    agenda::*,
    deadline_currentness::evaluate_deadline_currentness,
    deadline_tracking::TrackingPolicy,
    deadlines::DeadlineOverview,
    hearings::{HearingModality, HearingStatus, HearingStatusFilter, HearingTime},
};
use domain::{
    case_administration::CaseAdministrativeStatus,
    cases::CaseId,
    crypto::Sha256Digest,
    precautionary_hearings::{
        PrecautionaryHearingId, PrecautionaryHearingPurpose, PrecautionaryHearingRevision,
    },
    procedural_resources::ResourceId,
    resource_activities::ResourceActivityId,
    resource_hearings::{ResourceHearingId, ResourceHearingKind, ResourceHearingRevision},
};
use time::{Duration, OffsetDateTime, UtcOffset};
use uuid::Uuid;

#[path = "agenda_precautionary_hearings/access.rs"]
mod access;

fn case() -> AgendaCaseSummary {
    AgendaCaseSummary {
        case_id: CaseId::from_uuid(Uuid::from_u128(1)),
        title: "Authorized case".into(),
        reference: "CASE-1".into(),
        status: CaseAdministrativeStatus::Active,
    }
}

fn precautionary(at: OffsetDateTime, id: Uuid) -> AgendaItem {
    AgendaItem::PrecautionaryHearing {
        case: case(),
        hearing: Box::new(PrecautionaryHearingAgendaOverview {
            case_id: case().case_id,
            id: PrecautionaryHearingId::from_uuid(id),
            revision: PrecautionaryHearingRevision::new(2).unwrap(),
            purpose: PrecautionaryHearingPurpose::Review,
            scheduled_at: HearingTime::new(at).unwrap(),
            modality: HearingModality::Videoconference,
            status: HearingStatus::Scheduled,
            participant_count: 0,
            capture_digest: Sha256Digest::from_array([8; 32]),
        }),
    }
}

fn resource(at: OffsetDateTime, id: Uuid) -> AgendaItem {
    AgendaItem::ResourceHearing {
        case: case(),
        hearing: Box::new(ResourceHearingAgendaOverview {
            case_id: case().case_id,
            resource_id: ResourceId::from_uuid(Uuid::from_u128(4)),
            id: ResourceHearingId::from_uuid(id),
            revision: ResourceHearingRevision::initial(),
            kind: ResourceHearingKind::AppealArguments,
            scheduled_at: HearingTime::new(at).unwrap(),
            modality: HearingModality::InPerson,
            participant_count: 0,
            association_id: ResourceActivityId::from_uuid(Uuid::from_u128(5)),
            capture_digest: Sha256Digest::from_array([7; 32]),
        }),
    }
}

fn filtered(kind: AgendaKind, status: HearingStatusFilter) -> AgendaQuery {
    AgendaQuery::new(4, from(), until(), kind, status, None).unwrap()
}

#[test]
fn four_families_with_equal_instant_and_uuid_keep_order_and_cursor_continuation() {
    let base = technical::accepted(TrackingPolicy::Follow);
    let heads = technical::heads(&base);
    let current = evaluate_deadline_currentness(
        technical::inputs::hasher().as_ref(),
        &base,
        Some(&heads),
        checked_at(),
    )
    .unwrap();
    let deadline = DeadlineOverview::from(&current);
    let at = deadline.operational.due_at().unwrap();
    let id = deadline.id.as_uuid();
    let rows = vec![
        AgendaItem::Hearing(hearing(at, id.as_u128())),
        AgendaItem::Deadline {
            case: AgendaCaseSummary {
                case_id: deadline.case_id,
                ..case()
            },
            deadline: Box::new(deadline),
        },
        resource(at, id),
        precautionary(at, id),
    ];
    assert!(page(rows.clone()).validate(&query(4)).is_ok());
    let kinds = [
        AgendaItemKind::Hearing,
        AgendaItemKind::Deadline,
        AgendaItemKind::ResourceHearing,
        AgendaItemKind::PrecautionaryHearing,
    ];
    let mut after = None;
    for (index, row) in rows.iter().enumerate() {
        let key = row.key().unwrap();
        assert_eq!((key.kind(), key.id(), key.at()), (kinds[index], id, at));
        assert!(after.is_none_or(|prior| prior < key));
        let q = AgendaQuery::new(
            1,
            from(),
            until(),
            AgendaKind::All,
            HearingStatusFilter::Scheduled,
            after,
        )
        .unwrap();
        let complete = index == 3;
        let result = AgendaPage {
            checked_at: checked_at(),
            items: vec![row.clone()],
            complete,
            next_after: if complete { None } else { Some(key) },
        };
        assert!(result.validate(&q).is_ok());
        after = result.next_after;
    }
    assert!(after.is_none());
    let mut reversed = rows;
    reversed.swap(2, 3);
    assert!(page(reversed).validate(&query(4)).is_err());
}

#[test]
fn precautionary_filters_accept_current_revisions_and_distinguish_cancelled_appointments() {
    for status in [HearingStatus::Scheduled, HearingStatus::Cancelled] {
        let mut row = precautionary(from(), Uuid::from_u128(9));
        let AgendaItem::PrecautionaryHearing { hearing, .. } = &mut row else {
            unreachable!()
        };
        hearing.status = status;
        hearing.revision = PrecautionaryHearingRevision::new(3).unwrap();
        for kind in [AgendaKind::All, AgendaKind::PrecautionaryHearing] {
            for filter in [
                HearingStatusFilter::Scheduled,
                HearingStatusFilter::Cancelled,
                HearingStatusFilter::All,
            ] {
                let accepted =
                    filter == HearingStatusFilter::All || filter.status() == Some(status);
                assert_eq!(
                    page(vec![row.clone()])
                        .validate(&filtered(kind, filter))
                        .is_ok(),
                    accepted
                );
            }
        }
        for kind in [
            AgendaKind::Hearing,
            AgendaKind::Deadline,
            AgendaKind::ResourceHearing,
        ] {
            assert!(page(vec![row.clone()])
                .validate(&filtered(kind, HearingStatusFilter::Scheduled))
                .is_err());
        }
    }
    for row in [item(9), resource(from(), Uuid::from_u128(9))] {
        assert!(page(vec![row])
            .validate(&filtered(
                AgendaKind::PrecautionaryHearing,
                HearingStatusFilter::All,
            ))
            .is_err());
    }
}

#[test]
fn exact_precautionary_capture_purpose_revision_and_offset_survive_closed_case_projection() {
    let at = from().to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap());
    for purpose in [
        PrecautionaryHearingPurpose::Imposition,
        PrecautionaryHearingPurpose::Review,
    ] {
        for count in [0, 32] {
            let mut row = precautionary(at, Uuid::from_u128(9));
            let AgendaItem::PrecautionaryHearing { case, hearing } = &mut row else {
                unreachable!()
            };
            case.status = CaseAdministrativeStatus::Closed;
            hearing.purpose = purpose;
            hearing.participant_count = count;
            let expected = hearing.clone();
            assert!(page(vec![row.clone()]).validate(&query(1)).is_ok());
            assert_eq!(row.key().unwrap().at().offset(), UtcOffset::UTC);
            let AgendaItem::PrecautionaryHearing { hearing, .. } = row else {
                unreachable!()
            };
            assert_eq!(hearing, expected);
            assert_eq!(hearing.scheduled_at.value().offset(), at.offset());
            assert_eq!(hearing.revision.get(), 2);
            assert_eq!(hearing.capture_digest, Sha256Digest::from_array([8; 32]));
        }
    }
}

#[test]
fn precautionary_projection_rejects_foreign_case_invalid_metadata_and_oversized_participants() {
    for fault in 0..4 {
        let mut row = precautionary(from(), Uuid::from_u128(9));
        let AgendaItem::PrecautionaryHearing { case, hearing } = &mut row else {
            unreachable!()
        };
        match fault {
            0 => case.case_id = CaseId::new(),
            1 => case.title = " padded".into(),
            2 => case.reference.clear(),
            _ => hearing.participant_count = 33,
        }
        assert!(row.key().is_err(), "fault {fault}");
        assert!(page(vec![row]).validate(&query(1)).is_err());
    }
}

#[test]
fn precautionary_cursor_preserves_family_range_and_cancelled_continuation() {
    let id = Uuid::from_u128(9);
    let before = AgendaCursor::new(from(), AgendaItemKind::ResourceHearing, id).unwrap();
    let row = precautionary(from(), id);
    let q = AgendaQuery::new(
        1,
        from(),
        until(),
        AgendaKind::All,
        HearingStatusFilter::Scheduled,
        Some(before),
    )
    .unwrap();
    assert!(page(vec![row.clone()]).validate(&q).is_ok());
    let cursor = row.key().unwrap();
    for kind in [
        AgendaKind::Hearing,
        AgendaKind::Deadline,
        AgendaKind::ResourceHearing,
    ] {
        assert!(AgendaQuery::new(
            1,
            from(),
            until(),
            kind,
            HearingStatusFilter::Scheduled,
            Some(cursor),
        )
        .is_err());
    }
    let q = AgendaQuery::new(
        1,
        from(),
        until(),
        AgendaKind::PrecautionaryHearing,
        HearingStatusFilter::Cancelled,
        Some(cursor),
    )
    .unwrap();
    let next = AgendaCursor::new(
        from() + Duration::seconds(1),
        AgendaItemKind::PrecautionaryHearing,
        id,
    )
    .unwrap();
    let empty = AgendaPage {
        checked_at: checked_at(),
        items: vec![],
        complete: false,
        next_after: Some(next),
    };
    assert!(empty.validate(&q).is_ok());
    for at in [from() - Duration::seconds(1), until()] {
        assert!(page(vec![precautionary(at, id)])
            .validate(&query(1))
            .is_err());
    }
    let mut later = precautionary(from() + Duration::seconds(1), id);
    let AgendaItem::PrecautionaryHearing { hearing, .. } = &mut later else {
        unreachable!()
    };
    hearing.revision = PrecautionaryHearingRevision::new(3).unwrap();
    assert!(page(vec![row, later]).validate(&query(2)).is_err());
}
