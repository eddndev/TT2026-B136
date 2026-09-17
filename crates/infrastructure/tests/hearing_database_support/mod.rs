#![allow(dead_code)]

use application::cases::CaseRepository;
use application::hearings::*;
use application::identity::Principal;
use domain::case_administration::{CaseRevision, CaseStageRevision};
use domain::cases::CaseId;
use domain::identity::{Role, UserId};
use infrastructure::{PostgresHearingStore, RingSha256Hasher};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub use super::case_administration_support::Fixture;
use super::case_stage_database_support::{
    creation, processor, FixedClock, FormatCheck, TestIdentity,
};

pub fn complete(db: &mut Fixture) {
    db.case = CaseId::new();
    db.store()
        .register_penal(db.owner, db.case, creation(&db.case.to_string()), db.at)
        .unwrap();
}
pub fn store(db: &Fixture) -> Arc<PostgresHearingStore> {
    Arc::new(
        PostgresHearingStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}
pub fn service(db: &Fixture, actor: UserId, role: Role) -> HearingService {
    HearingService::new(
        store(db),
        Arc::new(TestIdentity(Principal {
            id: actor,
            email: "session@example.test".into(),
            role,
        })),
        processor(),
        Arc::new(FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}
pub fn context() -> HearingContextExpectation {
    HearingContextExpectation {
        case_revision: CaseRevision::FIRST,
        stage_revision: CaseStageRevision::FIRST,
    }
}
pub fn values(at: &str) -> HearingValues {
    HearingValues::new(HearingValuesInput {
        kind: HearingKind::Initial,
        scheduled_at: HearingTime::new(OffsetDateTime::parse(at, &Rfc3339).unwrap()).unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Court A").unwrap(),
        note: None,
        participants: vec![],
        conviction_basis: None,
    })
    .unwrap()
}
pub fn schedule() -> HearingCommand {
    HearingCommand {
        operation_id: HearingOperationId::new(),
        hearing_id: HearingId::new(),
        change: HearingChange::Schedule {
            context: context(),
            values: values("2026-09-15T09:00:00-06:00"),
        },
    }
}
pub fn persist(service: &HearingService, case: CaseId, command: HearingCommand) -> HearingDetail {
    let prepared = service.prepare("session", case, command.clone()).unwrap();
    service
        .submit("session", case, command, prepared.submission_digest)
        .unwrap()
}
pub fn counts(db: &mut Fixture) -> (i64, i64, i64) {
    let row = db
        .admin
        .query_one(
            "SELECT (SELECT count(*) FROM case_hearings),
        (SELECT count(*) FROM case_hearing_revisions),(SELECT count(*) FROM audit_events)",
            &[],
        )
        .unwrap();
    (row.get(0), row.get(1), row.get(2))
}
