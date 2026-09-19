use crate::{hearings::HearingStatusFilter, ApplicationError};
use domain::{clock::OffsetDateTime, typed_participants::Uuid};

pub const MAX_AGENDA_CANDIDATES: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgendaKind {
    All,
    Hearing,
    Deadline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgendaItemKind {
    Hearing,
    Deadline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AgendaCursor {
    at: OffsetDateTime,
    kind: AgendaItemKind,
    id: Uuid,
}
impl AgendaCursor {
    pub fn new(
        at: OffsetDateTime,
        kind: AgendaItemKind,
        id: Uuid,
    ) -> Result<Self, ApplicationError> {
        if !valid_instant(at) {
            return Err(invalid_query());
        }
        Ok(Self { at, kind, id })
    }
    pub const fn at(self) -> OffsetDateTime {
        self.at
    }
    pub const fn kind(self) -> AgendaItemKind {
        self.kind
    }
    pub const fn id(self) -> Uuid {
        self.id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgendaQuery {
    limit: u32,
    from: OffsetDateTime,
    until: OffsetDateTime,
    kind: AgendaKind,
    hearing_status: HearingStatusFilter,
    after: Option<AgendaCursor>,
}
impl AgendaQuery {
    pub fn new(
        limit: u32,
        from: OffsetDateTime,
        until: OffsetDateTime,
        kind: AgendaKind,
        hearing_status: HearingStatusFilter,
        after: Option<AgendaCursor>,
    ) -> Result<Self, ApplicationError> {
        if !(1..=100).contains(&limit)
            || !valid_instant(from)
            || !valid_instant(until)
            || from.nanosecond() != 0
            || until.nanosecond() != 0
            || until <= from
            || until - from > time::Duration::days(366)
            || (kind == AgendaKind::Deadline && hearing_status != HearingStatusFilter::Scheduled)
        {
            return Err(invalid_query());
        }
        let query = Self {
            limit,
            from,
            until,
            kind,
            hearing_status,
            after,
        };
        if after.is_some_and(|cursor| !query.accepts(cursor)) {
            return Err(invalid_query());
        }
        Ok(query)
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn from(self) -> OffsetDateTime {
        self.from
    }
    pub const fn until(self) -> OffsetDateTime {
        self.until
    }
    pub const fn kind(self) -> AgendaKind {
        self.kind
    }
    pub const fn hearing_status(self) -> HearingStatusFilter {
        self.hearing_status
    }
    pub const fn after(self) -> Option<AgendaCursor> {
        self.after
    }
}

impl AgendaQuery {
    pub(super) fn accepts(self, cursor: AgendaCursor) -> bool {
        valid_instant(cursor.at())
            && cursor.at() >= self.from
            && cursor.at() < self.until
            && matches!(
                (self.kind, cursor.kind()),
                (AgendaKind::All, _)
                    | (AgendaKind::Hearing, AgendaItemKind::Hearing)
                    | (AgendaKind::Deadline, AgendaItemKind::Deadline)
            )
    }
}
pub(super) fn valid_instant(at: OffsetDateTime) -> bool {
    at.offset().is_utc() && (1..=9999).contains(&at.year())
}
fn invalid_query() -> ApplicationError {
    ApplicationError::InvalidInput("invalid agenda range, limit, filter or cursor".into())
}
