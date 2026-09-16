use super::{
    receipt::inconsistent,
    validation::{validate_history, validate_page},
    *,
};
use crate::ApplicationError;
use domain::{crypto::Sha256Digest, identity::Permission};
impl JudicialCalendarWorkflow for JudicialCalendarService {
    fn list(
        &self,
        token: &str,
        query: JudicialCalendarQuery,
    ) -> Result<JudicialCalendarPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadJudicialCalendar)?;
        let page = self.store.list(actor, query.clone(), self.clock.now())?;
        validate_page(&page, &query)?;
        Ok(page)
    }
    fn get(
        &self,
        token: &str,
        id: JudicialCalendarId,
        revision: Option<JudicialCalendarRevision>,
    ) -> Result<JudicialCalendarDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ReadJudicialCalendar)?;
        let detail = self.store.get(actor, id, revision, self.clock.now())?;
        judicial_calendar_receipt_matches(self.hasher.as_ref(), &detail)?;
        if detail.id != id || revision.is_some_and(|v| v != detail.revision) {
            return Err(inconsistent(
                "calendar detail root or exact revision differs",
            ));
        }
        Ok(detail)
    }
    fn history(
        &self,
        token: &str,
        id: JudicialCalendarId,
        query: JudicialCalendarHistoryQuery,
    ) -> Result<JudicialCalendarHistoryPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadJudicialCalendar)?;
        let page = self.store.history(actor, id, query, self.clock.now())?;
        validate_history(self.hasher.as_ref(), id, &page, query)?;
        Ok(page)
    }
    fn days(
        &self,
        token: &str,
        id: JudicialCalendarId,
        revision: JudicialCalendarRevision,
        query: JudicialCalendarDaysQuery,
    ) -> Result<JudicialCalendarDays, ApplicationError> {
        let detail = self.get(token, id, Some(revision))?;
        let days = (query.from().days_since_epoch()..=query.through().days_since_epoch())
            .map(|day| {
                CivilDate::from_days_since_epoch(day).map(|date| detail.values.classify(date))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(JudicialCalendarDays {
            calendar_id: id,
            revision,
            values_digest: detail.values_digest,
            days,
        })
    }
    fn prepare(
        &self,
        token: &str,
        command: JudicialCalendarCommand,
    ) -> Result<JudicialCalendarDraft, ApplicationError> {
        self.prepare_command(token, command)
    }
    fn submit(
        &self,
        token: &str,
        command: JudicialCalendarCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<JudicialCalendarDetail, ApplicationError> {
        self.submit_command(token, command, expected_submission_digest)
    }
}
