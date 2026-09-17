use super::{authorization, decode, inconsistent, port, storage, PostgresJudicialCalendarStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{judicial_calendars::*, ApplicationError};
use domain::{clock::OffsetDateTime, identity::UserId};

impl PostgresJudicialCalendarStore {
    pub(super) fn list_page(
        &self,
        actor: UserId,
        query: JudicialCalendarQuery,
        at: OffsetDateTime,
    ) -> Result<JudicialCalendarPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, false)?;
        let after = query.after_id().map(|v| v.as_uuid());
        let status = query.status().status().map(|v| v.as_str());
        let jurisdiction = query.jurisdiction().map(|v| v.as_str());
        let limit = i64::from(query.limit()) + 1;
        let mut rows = tx
            .query(
                "SELECT calendar_id,revision FROM (
            SELECT DISTINCT ON (calendar_id) calendar_id,revision,status,values_view
            FROM judicial_calendar_revisions ORDER BY calendar_id,revision DESC) heads
            WHERE ($1::uuid IS NULL OR calendar_id>$1) AND ($2::text IS NULL OR status=$2)
            AND ($3::text IS NULL OR values_view#>>'{scope,jurisdiction}'=$3)
            AND ($4::text IS NULL OR values_view#>'{scope,entity_codes}' ? $4)
            ORDER BY calendar_id LIMIT $5",
                &[&after, &status, &jurisdiction, &query.entity_code(), &limit],
            )
            .map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let mut calendars = Vec::with_capacity(rows.len());
        for row in rows {
            let id = JudicialCalendarId::from_uuid(row.try_get(0).map_err(inconsistent)?);
            let revision = JudicialCalendarRevision::new(decode::counter(
                row.try_get(1).map_err(inconsistent)?,
            )?)
            .map_err(inconsistent)?;
            let detail = storage::detail(&mut tx, id, Some(revision), self.hasher.as_ref())?;
            calendars.push(JudicialCalendarOverview::from(&detail));
        }
        let next_after_id = if has_more {
            calendars.last().map(|v| v.id)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "judicial_calendar.list_read",
            "judicial_calendars",
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(JudicialCalendarPage {
            calendars,
            has_more,
            next_after_id,
        })
    }
    pub(super) fn history_page(
        &self,
        actor: UserId,
        id: JudicialCalendarId,
        query: JudicialCalendarHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<JudicialCalendarHistoryPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, false)?;
        storage::detail(&mut tx, id, None, self.hasher.as_ref())?;
        let before = query.before_revision().map(|v| i64::from(v.get()));
        let limit = i64::from(query.limit()) + 1;
        let mut rows = tx
            .query(
                "SELECT revision FROM judicial_calendar_revisions WHERE calendar_id=$1
            AND ($2::bigint IS NULL OR revision<$2) ORDER BY revision DESC LIMIT $3",
                &[&id.as_uuid(), &before, &limit],
            )
            .map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let mut revisions = Vec::with_capacity(rows.len());
        for row in rows {
            let revision = JudicialCalendarRevision::new(decode::counter(
                row.try_get(0).map_err(inconsistent)?,
            )?)
            .map_err(inconsistent)?;
            let detail = storage::detail(&mut tx, id, Some(revision), self.hasher.as_ref())?;
            revisions.push(JudicialCalendarHistoryEntry::from(&detail));
        }
        let next_before_revision = if has_more {
            revisions.last().map(|v| v.revision)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "judicial_calendar.history_read",
            &format!("calendar:{id}:history"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(JudicialCalendarHistoryPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
}
