use super::{authorization, port, preparation, storage, PostgresJudicialCalendarStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{judicial_calendars::*, ApplicationError};
use domain::{clock::OffsetDateTime, identity::UserId};

impl JudicialCalendarStore for PostgresJudicialCalendarStore {
    fn list(
        &self,
        actor: UserId,
        query: JudicialCalendarQuery,
        at: OffsetDateTime,
    ) -> Result<JudicialCalendarPage, ApplicationError> {
        self.list_page(actor, query, at)
    }
    fn get(
        &self,
        actor: UserId,
        id: JudicialCalendarId,
        revision: Option<JudicialCalendarRevision>,
        at: OffsetDateTime,
    ) -> Result<JudicialCalendarDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, false)?;
        let detail = storage::detail(&mut tx, id, revision, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            "judicial_calendar.read",
            &format!(
                "calendar:{id}:revision:{}:sha256:{}",
                detail.revision.get(),
                detail.values_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
    fn history(
        &self,
        actor: UserId,
        id: JudicialCalendarId,
        query: JudicialCalendarHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<JudicialCalendarHistoryPage, ApplicationError> {
        self.history_page(actor, id, query, at)
    }
    fn prepare(
        &self,
        actor: UserId,
        command: &JudicialCalendarCommand,
    ) -> Result<JudicialCalendarPreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorization::actor(&mut tx, actor, true)?;
        let result = preparation::load(&mut tx, command, self.hasher.as_ref())?;
        tx.rollback().map_err(port)?;
        Ok(result)
    }
    fn commit(
        &self,
        actor: UserId,
        prepared: PreparedJudicialCalendarChange,
    ) -> Result<JudicialCalendarDetail, ApplicationError> {
        self.commit_change(actor, prepared)
    }
}
