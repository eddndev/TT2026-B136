#![allow(dead_code)]
use application::{agenda::*, hearings::*};
use domain::{case_administration::CaseAdministrativeStatus, cases::CaseId};
use time::{macros::datetime, OffsetDateTime};
use uuid::Uuid;

pub fn checked_at() -> OffsetDateTime {
    datetime!(2027-02-01 12:00:00.123456789 UTC)
}

pub fn from() -> OffsetDateTime {
    datetime!(2026-01-01 00:00 UTC)
}

pub fn until() -> OffsetDateTime {
    datetime!(2027-01-01 00:00 UTC)
}

pub fn query(limit: u32) -> AgendaQuery {
    AgendaQuery::new(
        limit,
        from(),
        until(),
        AgendaKind::All,
        HearingStatusFilter::Scheduled,
        None,
    )
    .unwrap()
}

pub fn hearing(at: OffsetDateTime, id: u128) -> HearingOverview {
    HearingOverview {
        case_id: CaseId::from_uuid(Uuid::from_u128(1)),
        case_title: "Authorized case".into(),
        case_reference: "CASE-1".into(),
        case_status: CaseAdministrativeStatus::Active,
        id: HearingId::from_uuid(Uuid::from_u128(id)),
        revision: HearingRevision::initial(),
        kind: HearingKind::Initial,
        scheduled_at: HearingTime::new(at).unwrap(),
        modality: HearingModality::InPerson,
        status: HearingStatus::Scheduled,
        participant_count: 0,
    }
}

pub fn item(id: u128) -> AgendaItem {
    AgendaItem::Hearing(hearing(from(), id))
}

pub fn cursor(id: u128) -> AgendaCursor {
    AgendaCursor::new(from(), AgendaItemKind::Hearing, Uuid::from_u128(id)).unwrap()
}

pub fn page(items: Vec<AgendaItem>) -> AgendaPage {
    AgendaPage {
        checked_at: checked_at(),
        items,
        complete: true,
        next_after: None,
    }
}
