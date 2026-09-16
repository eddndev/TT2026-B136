#![allow(dead_code)]
pub use super::case_administration_support::Fixture;
use super::case_stage_database_support::FixedClock;
use application::hearing_results::*;
use domain::{clock::Clock, hearings::HearingId};
use infrastructure::{PostgresHearingResultStore, RingSha256Hasher};
use std::sync::Arc;
use time::{Date, Month, UtcOffset};

pub fn store(db: &Fixture) -> Arc<PostgresHearingResultStore> {
    Arc::new(
        PostgresHearingResultStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}
pub fn clocked_store(
    db: &Fixture,
    clock: Arc<dyn Clock + Send + Sync>,
) -> Arc<PostgresHearingResultStore> {
    Arc::new(
        PostgresHearingResultStore::open(&db.runtime_url, Arc::new(RingSha256Hasher), clock)
            .unwrap(),
    )
}
pub fn values(summary: &str) -> HearingResultValues {
    HearingResultValues::new(HearingResultValuesInput {
        occurrence: HearingResultOccurrence::Occurred,
        extent: HearingResultExtent::Partial,
        event_time: DeclaredHearingResultTime::date(
            Date::from_calendar_date(2024, Month::December, 31).unwrap(),
            UtcOffset::UTC,
        )
        .unwrap(),
        summary: HearingResultText::new(summary).unwrap(),
        attendees: vec![],
        agreements: vec![],
        provenance: HearingResultProvenance::new(
            HearingResultProvenanceKind::OperatorNote,
            None,
            None,
        )
        .unwrap(),
    })
    .unwrap()
}
pub fn record(hearing: HearingId) -> HearingResultCommand {
    HearingResultCommand {
        operation_id: HearingResultOperationId::new(),
        hearing_id: hearing,
        result_id: HearingResultId::new(),
        change: HearingResultChange::Record {
            anchor_revision: domain::hearings::HearingRevision::initial(),
            continuation: None,
            values: values("Declared session"),
        },
    }
}
pub fn counts(db: &mut Fixture) -> (i64, i64, i64) {
    let row=db.admin.query_one("SELECT (SELECT count(*) FROM case_hearing_results),(SELECT count(*) FROM case_hearing_result_revisions),(SELECT count(*) FROM audit_events)",&[]).unwrap();
    (row.get(0), row.get(1), row.get(2))
}

pub fn service(
    db: &Fixture,
    actor: domain::identity::UserId,
    role: domain::identity::Role,
) -> HearingResultService {
    HearingResultService::new(
        store(db),
        Arc::new(super::case_stage_database_support::TestIdentity(
            application::identity::Principal {
                id: actor,
                email: "session@example.test".into(),
                role,
            },
        )),
        super::case_stage_database_support::processor(),
        Arc::new(super::case_stage_database_support::FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}
pub fn persist(
    service: &HearingResultService,
    case: domain::cases::CaseId,
    command: HearingResultCommand,
) -> HearingResultDetail {
    let prepared = service.prepare("session", case, command.clone()).unwrap();
    service
        .submit("session", case, command, prepared.submission_digest)
        .unwrap()
}
