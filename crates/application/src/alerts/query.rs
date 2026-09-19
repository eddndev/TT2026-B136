use super::{AlertError, AlertId};
use crate::ApplicationError;
use domain::{clock::OffsetDateTime, typed_participants::Uuid};

pub const MAX_ALERT_CANDIDATES: u32 = 100;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertReadFilter {
    All,
    Unread,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertStateFilter {
    Active,
    All,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertCursor {
    created_at: OffsetDateTime,
    id: AlertId,
    read: AlertReadFilter,
    state: AlertStateFilter,
}
impl AlertCursor {
    pub fn new(
        created_at: OffsetDateTime,
        id: AlertId,
        read: AlertReadFilter,
        state: AlertStateFilter,
    ) -> Result<Self, ApplicationError> {
        if !super::validation::utc_time(created_at) {
            return Err(invalid_cursor());
        }
        Ok(Self {
            created_at,
            id,
            read,
            state,
        })
    }
    pub const fn created_at(self) -> OffsetDateTime {
        self.created_at
    }
    pub const fn id(self) -> AlertId {
        self.id
    }
    pub const fn read_filter(self) -> AlertReadFilter {
        self.read
    }
    pub const fn state_filter(self) -> AlertStateFilter {
        self.state
    }
    pub(super) fn key(self) -> (i64, u32, Uuid) {
        (
            self.created_at.unix_timestamp(),
            self.created_at.nanosecond(),
            self.id.as_uuid(),
        )
    }
    pub fn encode(self) -> String {
        let read = match self.read {
            AlertReadFilter::All => "all",
            AlertReadFilter::Unread => "unread",
        };
        let state = match self.state {
            AlertStateFilter::Active => "active",
            AlertStateFilter::All => "all",
        };
        format!(
            "a1:{}:{:09}:{}:{read}:{state}",
            self.created_at.unix_timestamp(),
            self.created_at.nanosecond(),
            self.id
        )
    }
    pub fn parse(value: &str) -> Result<Self, ApplicationError> {
        if value.len() > 128 || !value.is_ascii() {
            return Err(invalid_cursor());
        }
        let fields: Vec<_> = value.split(':').collect();
        let ["a1", seconds, nanos, id, read, state] = fields.as_slice() else {
            return Err(invalid_cursor());
        };
        let seconds = seconds.parse::<i64>().map_err(|_| invalid_cursor())?;
        let nanos = nanos.parse::<u32>().map_err(|_| invalid_cursor())?;
        let created_at = OffsetDateTime::from_unix_timestamp(seconds)
            .map_err(|_| invalid_cursor())?
            .replace_nanosecond(nanos)
            .map_err(|_| invalid_cursor())?;
        let id = AlertId::from_uuid(Uuid::parse_str(id).map_err(|_| invalid_cursor())?);
        let read = match *read {
            "all" => AlertReadFilter::All,
            "unread" => AlertReadFilter::Unread,
            _ => return Err(invalid_cursor()),
        };
        let state = match *state {
            "active" => AlertStateFilter::Active,
            "all" => AlertStateFilter::All,
            _ => return Err(invalid_cursor()),
        };
        let result = Self::new(created_at, id, read, state)?;
        if result.encode() != value {
            return Err(invalid_cursor());
        }
        Ok(result)
    }
}
fn invalid_cursor() -> ApplicationError {
    AlertError::Invalid("cursor").into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertQuery {
    limit: u32,
    read: AlertReadFilter,
    state: AlertStateFilter,
    after: Option<AlertCursor>,
}
impl AlertQuery {
    pub fn new(
        limit: u32,
        read: AlertReadFilter,
        state: AlertStateFilter,
        after: Option<AlertCursor>,
    ) -> Result<Self, ApplicationError> {
        if !(1..=MAX_ALERT_CANDIDATES).contains(&limit) {
            return Err(AlertError::Invalid("page limit").into());
        }
        if after.is_some_and(|cursor| cursor.read != read || cursor.state != state) {
            return Err(invalid_cursor());
        }
        Ok(Self {
            limit,
            read,
            state,
            after,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn read_filter(self) -> AlertReadFilter {
        self.read
    }
    pub const fn state_filter(self) -> AlertStateFilter {
        self.state
    }
    pub const fn after(self) -> Option<AlertCursor> {
        self.after
    }
}
