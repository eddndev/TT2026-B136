pub use crate::case_administration_support::Fixture;
pub use application::{
    agenda::*,
    cases::{CaseRepository, CaseRevisionExpectation},
    hearings::HearingStatusFilter,
    precautionary_hearings::*,
};
pub use domain::{
    case_administration::CaseAdministrativeStatus,
    hearings::{HearingStatus, HearingTime},
    identity::Role,
    precautionary_hearings::*,
};
pub use infrastructure::{PostgresAgendaStore, RingSha256Hasher};
pub use std::sync::Arc;
pub use time::{Duration, OffsetDateTime};

pub fn store(db: &Fixture) -> PostgresAgendaStore {
    PostgresAgendaStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(crate::case_stage_database_support::FixedClock(db.at)),
    )
    .unwrap()
}
pub fn query(
    at: OffsetDateTime,
    limit: u32,
    kind: AgendaKind,
    status: HearingStatusFilter,
    after: Option<AgendaCursor>,
) -> AgendaQuery {
    let at = at.to_offset(time::UtcOffset::UTC);
    AgendaQuery::new(limit, at, at + Duration::seconds(1), kind, status, after).unwrap()
}
pub fn appointment(item: &AgendaItem) -> &PrecautionaryHearingAgendaOverview {
    let AgendaItem::PrecautionaryHearing { case, hearing } = item else {
        panic!("expected a precautionary appointment")
    };
    assert_eq!(case.case_id, hearing.case_id);
    hearing
}
pub fn scheduled(command: &PrecautionaryHearingCommand) -> OffsetDateTime {
    let PrecautionaryHearingChange::Schedule { values, .. } = &command.change else {
        panic!("expected a scheduling instruction")
    };
    values.scheduled_at().utc()
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
