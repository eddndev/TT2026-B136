use super::*;
use crate::ApplicationError;
use domain::{
    alerts::AlertWindow, cases::CaseMetadata, clock::OffsetDateTime, identity::UserId,
    procedural_facts::FactLabel,
};
use std::collections::HashSet;
use time::UtcOffset;

pub(super) fn stored(message: &str) -> ApplicationError {
    AlertError::Stored(message.into()).into()
}
pub(super) fn utc_time(value: OffsetDateTime) -> bool {
    value.offset() == UtcOffset::UTC && (1..=9999).contains(&value.year())
}
fn recorded_between(value: OffsetDateTime, start: OffsetDateTime, end: OffsetDateTime) -> bool {
    utc_time(value) && value >= start && value <= end
}

impl AlertRecord {
    pub fn validate(
        &self,
        actor: UserId,
        checked_at: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        if self.recipient_id != actor || self.origin.revision == 0 {
            return Err(stored("notification identity differs"));
        }
        let label = FactLabel::new(&self.subject_title).map_err(|_| stored("subject title"))?;
        let case = CaseMetadata::new(&self.case_title, &self.case_reference)
            .map_err(|_| stored("case context"))?;
        if label.as_str() != self.subject_title
            || case.title() != self.case_title
            || case.reference() != self.case_reference
        {
            return Err(stored("notification context is not normalized"));
        }
        if !utc_time(checked_at)
            || !utc_time(self.trigger_at)
            || !recorded_between(self.created_at, self.trigger_at, checked_at)
            || self
                .read_at
                .is_some_and(|at| !recorded_between(at, self.created_at, checked_at))
        {
            return Err(stored("notification chronology differs"));
        }
        if let AlertState::Resolved { at, .. } = self.state {
            if !recorded_between(at, self.created_at, checked_at) {
                return Err(stored("notification resolution chronology differs"));
            }
        }
        if let AlertEmailStatus::Accepted { accepted_at } = self.email {
            if !recorded_between(accepted_at, self.created_at, checked_at) {
                return Err(stored("email acceptance chronology differs"));
            }
        }
        if matches!(self.subject, AlertSubject::Hearing { .. })
            && !matches!(self.kind, AlertKind::Upcoming { .. })
        {
            return Err(stored("notification kind does not apply to hearing"));
        }
        match self.kind {
            AlertKind::Upcoming {
                lead_hours,
                activity_at,
            } => {
                let window = AlertWindow::upcoming(activity_at, lead_hours)
                    .map_err(|_| stored("notification anticipation"))?;
                if !utc_time(activity_at)
                    || self.trigger_at != window.starts_at()
                    || !window.contains(self.created_at)
                {
                    return Err(stored("notification anticipation chronology differs"));
                }
            }
            AlertKind::OverdueUnattended { due_at } => {
                if !utc_time(due_at) || self.trigger_at != due_at {
                    return Err(stored("overdue notification instant differs"));
                }
            }
            AlertKind::DueChangedSoon {
                previous_due_at,
                current_due_at,
            } => {
                if !utc_time(previous_due_at)
                    || !utc_time(current_due_at)
                    || previous_due_at == current_due_at
                {
                    return Err(stored("notification due transition differs"));
                }
            }
            AlertKind::ReviewRequired => (),
        }
        Ok(())
    }
}

impl AlertPage {
    pub fn validate(&self, actor: UserId, query: &AlertQuery) -> Result<(), ApplicationError> {
        if !utc_time(self.checked_at)
            || self.alerts.len() > query.limit() as usize
            || self.has_more != self.next_cursor.is_some()
        {
            return Err(stored("notification page bounds differ"));
        }
        let mut previous = query.after().map(AlertCursor::key);
        let mut ids = HashSet::new();
        for alert in &self.alerts {
            alert.validate(actor, self.checked_at)?;
            if !ids.insert(alert.id)
                || (query.read_filter() == AlertReadFilter::Unread && alert.read_at.is_some())
                || (query.state_filter() == AlertStateFilter::Active
                    && alert.state != AlertState::Active)
            {
                return Err(stored("notification page selection differs"));
            }
            let key = (
                alert.created_at.unix_timestamp(),
                alert.created_at.nanosecond(),
                alert.id.as_uuid(),
            );
            if previous.is_some_and(|previous| key >= previous) {
                return Err(stored("notification page order differs"));
            }
            previous = Some(key);
        }
        if let Some(cursor) = self.next_cursor {
            if cursor.read_filter() != query.read_filter()
                || cursor.state_filter() != query.state_filter()
                || cursor.created_at() > self.checked_at
                || query
                    .after()
                    .is_some_and(|after| cursor.key() >= after.key())
                || previous.is_some_and(|last| cursor.key() > last)
            {
                return Err(stored("notification continuation differs"));
            }
        }
        Ok(())
    }
}
