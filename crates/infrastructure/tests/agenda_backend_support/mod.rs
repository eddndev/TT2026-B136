#![allow(dead_code)]

use crate::{
    case_stage_database_support::FixedClock, deadline_backend_support as dl,
    hearing_database_support as hearings,
};
use application::{agenda::*, deadlines::*, hearings::*, procedural_facts::*};
use domain::identity::Role;
use infrastructure::{PostgresAgendaStore, RingSha256Hasher};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

pub use crate::case_administration_support::Fixture;

pub fn fixture() -> Option<Fixture> {
    let mut db = Fixture::new()?;
    hearings::complete(&mut db);
    Some(db)
}

pub fn store(db: &Fixture) -> PostgresAgendaStore {
    PostgresAgendaStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
    .unwrap()
}

pub fn query(limit: u32, kind: AgendaKind, after: Option<AgendaCursor>) -> AgendaQuery {
    AgendaQuery::new(
        limit,
        OffsetDateTime::parse("2026-01-01T00:00:00Z", &Rfc3339).unwrap(),
        OffsetDateTime::parse("2027-01-01T00:00:00Z", &Rfc3339).unwrap(),
        kind,
        HearingStatusFilter::Scheduled,
        after,
    )
    .unwrap()
}

pub fn accepted(db: &Fixture, id: u128) -> (FactDetail, DeadlineDetail) {
    let profile = dl::profile(db);
    let source = dl::source(db);
    let mut command = dl::command(db, &profile, &source);
    command.deadline_id = DeadlineId::from_uuid(Uuid::from_u128(id));
    let base = dl::persist(
        &dl::service(db, db.owner, Role::Owner),
        db.case,
        dl::human(command, Some(dl::FOLLOW_RESOLUTION)),
    );
    assert!(base.calculation.result.due_at().is_some());
    (source, base)
}

pub fn hearing_at(db: &Fixture, id: Uuid, at: OffsetDateTime) -> HearingDetail {
    let mut command = hearings::schedule();
    command.hearing_id = HearingId::from_uuid(id);
    let HearingChange::Schedule { values, .. } = &mut command.change else {
        unreachable!()
    };
    *values = hearings::values(&at.format(&Rfc3339).unwrap());
    hearings::persist(
        &hearings::service(db, db.owner, Role::Owner),
        db.case,
        command,
    )
}

pub fn audits(db: &mut Fixture) -> i64 {
    db.admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='agenda.read'",
            &[],
        )
        .unwrap()
        .get(0)
}

pub fn unchanged_resources(db: &mut Fixture) -> serde_json::Value {
    db.admin
        .query_one(
            "SELECT jsonb_build_object(
                'hearings',(SELECT jsonb_agg(to_jsonb(r) ORDER BY hearing_id,revision)
                    FROM case_hearing_revisions r),
                'deadlines',(SELECT jsonb_agg(to_jsonb(r) ORDER BY deadline_id,revision)
                    FROM case_deadline_revisions r),
                'events',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence)
                    FROM deadline_source_events r))",
            &[],
        )
        .unwrap()
        .get(0)
}
