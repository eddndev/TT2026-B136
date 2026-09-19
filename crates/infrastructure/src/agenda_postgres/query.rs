use super::{
    authorization,
    header::{kind_rank, Header},
    inconsistent, port, projection, PostgresAgendaStore,
};
use application::{agenda::*, hearings::HearingStatusFilter, ApplicationError};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use std::collections::HashMap;
use time::UtcOffset;

const SELECT_HEADERS: &str = include_str!("selection.sql");

impl AgendaStore for PostgresAgendaStore {
    fn list(&self, actor: UserId, query: AgendaQuery) -> Result<AgendaPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, query.kind())?;
        let checked_at = self.clock.now().to_offset(UtcOffset::UTC);
        if !(1..=9999).contains(&checked_at.year()) {
            return Err(inconsistent("agenda clock is outside supported years"));
        }
        let kind = match query.kind() {
            AgendaKind::All => -1_i16,
            AgendaKind::Hearing => 0,
            AgendaKind::Deadline => 1,
        };
        let status = query.hearing_status().status().map(|value| value.as_str());
        let after = query.after();
        let after_seconds = after.map(|value| value.at().unix_timestamp());
        let after_nanos = after.map(|value| value.at().nanosecond() as i32);
        let after_kind = after.map(|value| kind_rank(value.kind()));
        let after_id = after.map(|value| value.id());
        let rows = tx
            .query(
                SELECT_HEADERS,
                &[
                    &(principal.role == Role::Owner),
                    &principal.id.as_uuid(),
                    &kind,
                    &status,
                    &query.from().unix_timestamp(),
                    &query.until().unix_timestamp(),
                    &after_seconds,
                    &after_nanos,
                    &after_kind,
                    &after_id,
                ],
            )
            .map_err(port)?;
        let mut cases: HashMap<CaseId, AgendaCaseSummary> = HashMap::new();
        let mut items = Vec::new();
        let mut examined = 0;
        let mut last = None;
        for row in rows.iter().take(MAX_AGENDA_CANDIDATES) {
            if items.len() == query.limit() as usize {
                break;
            }
            let header = Header::decode(row)?;
            let case = match cases.get(&header.case) {
                Some(case) => case.clone(),
                None => {
                    let case = authorization::case_summary(
                        &mut tx,
                        &principal,
                        header.case,
                        self.hasher.as_ref(),
                    )?;
                    cases.insert(header.case, case.clone());
                    case
                }
            };
            if let Some(item) =
                projection::item(&mut tx, &header, case, self.hasher.as_ref(), checked_at)?
            {
                items.push(item);
            }
            examined += 1;
            last = Some(header.key);
        }
        let complete = examined == rows.len();
        let page = AgendaPage {
            checked_at,
            items,
            complete,
            next_after: if complete { None } else { last },
        };
        page.validate(&query)?;
        crate::audit_postgres::append_transaction(
            &mut tx,
            &principal.email,
            "agenda.read",
            &resource(query, &page),
            checked_at,
        )?;
        tx.commit().map_err(port)?;
        Ok(page)
    }
}

fn resource(query: AgendaQuery, page: &AgendaPage) -> String {
    let kind = match query.kind() {
        AgendaKind::All => "all",
        AgendaKind::Hearing => "hearing",
        AgendaKind::Deadline => "deadline",
    };
    let status = match query.hearing_status() {
        HearingStatusFilter::Scheduled => "scheduled",
        HearingStatusFilter::Cancelled => "cancelled",
        HearingStatusFilter::All => "all",
    };
    format!(
        "agenda:from:{}:until:{}:kind:{kind}:hearing_status:{status}:after:{}:next:{}:complete:{}",
        query.from().unix_timestamp(),
        query.until().unix_timestamp(),
        cursor(query.after()),
        cursor(page.next_after),
        page.complete,
    )
}

fn cursor(value: Option<AgendaCursor>) -> String {
    value.map_or_else(
        || "none".into(),
        |value| {
            format!(
                "{},{},{},{}",
                value.at().unix_timestamp(),
                value.at().nanosecond(),
                kind_rank(value.kind()),
                value.id(),
            )
        },
    )
}
