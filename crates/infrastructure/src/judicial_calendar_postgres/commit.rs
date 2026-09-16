use super::{authorization, inconsistent, port, preparation, write, PostgresJudicialCalendarStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{judicial_calendars::*, ApplicationError};
use domain::identity::UserId;
use time::UtcOffset;

impl PostgresJudicialCalendarStore {
    pub(super) fn commit_change(
        &self,
        actor: UserId,
        prepared: PreparedJudicialCalendarChange,
    ) -> Result<JudicialCalendarDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, true)?;
        if actor != prepared.actor() {
            return Err(ApplicationError::InvalidSession);
        }
        let command = prepared.command();
        let observed = preparation::load(&mut tx, command, self.hasher.as_ref())?;
        if observed != *prepared.preparation() {
            return Err(JudicialCalendarError::RevisionConflict.into());
        }
        let values = match &command.change {
            JudicialCalendarChange::Publish { values }
            | JudicialCalendarChange::Replace { values, .. } => values,
            JudicialCalendarChange::Retire { .. } => {
                &observed
                    .base
                    .as_ref()
                    .ok_or(JudicialCalendarError::NotFound)?
                    .values
            }
        };
        if values != prepared.values()
            || judicial_calendar_values_digest(self.hasher.as_ref(), values)
                != prepared.values_digest()
            || judicial_calendar_submission_digest(
                self.hasher.as_ref(),
                actor,
                command,
                prepared.values_digest(),
            ) != prepared.submission_digest()
        {
            return Err(inconsistent(
                "prepared calendar does not match the locked sources",
            ));
        }
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        if !(1..=9999).contains(&at.year()) {
            return Err(inconsistent(
                "calendar capture clock is outside supported years",
            ));
        }
        let detail = JudicialCalendarDetail {
            id: command.calendar_id,
            revision: command.result_revision()?,
            values: prepared.values().clone(),
            values_digest: prepared.values_digest(),
            status: command.result_status(),
            reason: command.reason().cloned(),
            receipt: JudicialCalendarReceipt {
                operation_id: command.operation_id,
                action: command.action(),
                expected_revision: command.expected_revision(),
                submission_digest: prepared.submission_digest(),
            },
            recorded_at: at,
            recorded_by: JudicialCalendarActorSnapshot {
                id: principal.id,
                email: principal.email.clone(),
            },
        };
        judicial_calendar_receipt_matches(self.hasher.as_ref(), &detail)?;
        write::insert(&mut tx, &detail, command)?;
        let action = match command.action() {
            JudicialCalendarAction::Publish => "judicial_calendar.published",
            JudicialCalendarAction::Replace => "judicial_calendar.replaced",
            JudicialCalendarAction::Retire => "judicial_calendar.retired",
        };
        append_transaction(
            &mut tx,
            &principal.email,
            action,
            &format!(
                "calendar:{}:revision:{}:operation:{}:sha256:{}",
                detail.id,
                detail.revision.get(),
                command.operation_id,
                detail.receipt.submission_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
}
