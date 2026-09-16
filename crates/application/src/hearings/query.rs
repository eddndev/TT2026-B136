use super::{HearingId, HearingRevision, HearingStatus, HearingTime};
use crate::ApplicationError;
use domain::clock::OffsetDateTime;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum HearingStatusFilter {
    #[default]
    Scheduled,
    Cancelled,
    All,
}
impl HearingStatusFilter {
    pub const fn status(self) -> Option<HearingStatus> {
        match self {
            Self::Scheduled => Some(HearingStatus::Scheduled),
            Self::Cancelled => Some(HearingStatus::Cancelled),
            Self::All => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingQuery {
    limit: u32,
    after_id: Option<HearingId>,
    status: HearingStatusFilter,
}
impl HearingQuery {
    pub fn new(
        limit: u32,
        after_id: Option<HearingId>,
        status: HearingStatusFilter,
    ) -> Result<Self, ApplicationError> {
        validate_limit(limit)?;
        Ok(Self {
            limit,
            after_id,
            status,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn after_id(self) -> Option<HearingId> {
        self.after_id
    }
    pub const fn status(self) -> HearingStatusFilter {
        self.status
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingHistoryQuery {
    limit: u32,
    before_revision: Option<HearingRevision>,
}
impl HearingHistoryQuery {
    pub fn new(limit: u32, before_revision: Option<u32>) -> Result<Self, ApplicationError> {
        validate_limit(limit)?;
        Ok(Self {
            limit,
            before_revision: before_revision.map(HearingRevision::new).transpose()?,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn before_revision(self) -> Option<HearingRevision> {
        self.before_revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingAgendaCursor {
    pub at: HearingTime,
    pub id: HearingId,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingAgendaQuery {
    limit: u32,
    from: OffsetDateTime,
    until: OffsetDateTime,
    after: Option<HearingAgendaCursor>,
    status: HearingStatusFilter,
}
impl HearingAgendaQuery {
    pub fn new(
        limit: u32,
        from: OffsetDateTime,
        until: OffsetDateTime,
        after: Option<HearingAgendaCursor>,
        status: HearingStatusFilter,
    ) -> Result<Self, ApplicationError> {
        validate_limit(limit)?;
        for at in [from, until] {
            HearingTime::new(at)?;
            if !at.offset().is_utc() {
                return Err(invalid_query());
            }
        }
        if until <= from || (until - from).whole_seconds() > 366 * 86400 {
            return Err(invalid_query());
        }
        if after.is_some_and(|cursor| {
            !cursor.at.value().offset().is_utc()
                || cursor.at.utc() < from
                || cursor.at.utc() >= until
        }) {
            return Err(invalid_query());
        }
        Ok(Self {
            limit,
            from,
            until,
            after,
            status,
        })
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
    pub const fn after(self) -> Option<HearingAgendaCursor> {
        self.after
    }
    pub const fn status(self) -> HearingStatusFilter {
        self.status
    }
}
fn validate_limit(limit: u32) -> Result<(), ApplicationError> {
    if !(1..=100).contains(&limit) {
        return Err(invalid_query());
    }
    Ok(())
}
fn invalid_query() -> ApplicationError {
    ApplicationError::InvalidInput("invalid hearing page or bounded UTC interval".into())
}
