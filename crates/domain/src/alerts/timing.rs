use time::{Duration, OffsetDateTime, UtcOffset};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AlertTimingError {
    #[error("alert anticipation must be between 1 and 720 hours")]
    InvalidLeadHours,
    #[error("alert anticipations must contain at most eight distinct values")]
    InvalidAnticipations,
    #[error("alert timing must stay within UTC years 1 through 9999")]
    UnsupportedTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlertLeadHours(u16);

impl AlertLeadHours {
    pub fn new(hours: u16) -> Result<Self, AlertTimingError> {
        if !(1..=720).contains(&hours) {
            return Err(AlertTimingError::InvalidLeadHours);
        }
        Ok(Self(hours))
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertAnticipations(Vec<AlertLeadHours>);

impl AlertAnticipations {
    pub fn new(mut hours: Vec<AlertLeadHours>) -> Result<Self, AlertTimingError> {
        if hours.len() > 8 {
            return Err(AlertTimingError::InvalidAnticipations);
        }
        hours.sort_unstable_by(|left, right| right.cmp(left));
        if hours.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(AlertTimingError::InvalidAnticipations);
        }
        Ok(Self(hours))
    }

    pub fn hours(&self) -> &[AlertLeadHours] {
        &self.0
    }
}

impl Default for AlertAnticipations {
    fn default() -> Self {
        Self(vec![AlertLeadHours(48), AlertLeadHours(24)])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertWindow {
    starts_at: OffsetDateTime,
    ends_at: Option<OffsetDateTime>,
}

impl AlertWindow {
    pub fn upcoming(
        activity_at: OffsetDateTime,
        lead: AlertLeadHours,
    ) -> Result<Self, AlertTimingError> {
        let activity_at = checked_utc(activity_at)?;
        let starts_at = activity_at
            .checked_sub(Duration::hours(i64::from(lead.get())))
            .ok_or(AlertTimingError::UnsupportedTime)?;
        Ok(Self {
            starts_at: checked_utc(starts_at)?,
            ends_at: Some(activity_at),
        })
    }

    pub fn overdue(activity_at: OffsetDateTime) -> Result<Self, AlertTimingError> {
        Ok(Self {
            starts_at: checked_utc(activity_at)?,
            ends_at: None,
        })
    }

    pub const fn starts_at(self) -> OffsetDateTime {
        self.starts_at
    }

    pub const fn ends_at(self) -> Option<OffsetDateTime> {
        self.ends_at
    }

    pub fn contains(self, now: OffsetDateTime) -> bool {
        now >= self.starts_at && self.ends_at.is_none_or(|end| now < end)
    }
}

fn checked_utc(value: OffsetDateTime) -> Result<OffsetDateTime, AlertTimingError> {
    let value = value.to_offset(UtcOffset::UTC);
    if !(1..=9999).contains(&value.year()) {
        return Err(AlertTimingError::UnsupportedTime);
    }
    Ok(value)
}
