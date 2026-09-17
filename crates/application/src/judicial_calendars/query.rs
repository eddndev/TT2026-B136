use super::*;
use crate::ApplicationError;
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum JudicialCalendarStatusFilter {
    All,
    #[default]
    Published,
    Retired,
}
impl JudicialCalendarStatusFilter {
    pub const fn status(self) -> Option<JudicialCalendarStatus> {
        match self {
            Self::All => None,
            Self::Published => Some(JudicialCalendarStatus::Published),
            Self::Retired => Some(JudicialCalendarStatus::Retired),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarQuery {
    limit: u32,
    after_id: Option<JudicialCalendarId>,
    status: JudicialCalendarStatusFilter,
    jurisdiction: Option<JudicialCalendarJurisdiction>,
    entity_code: Option<String>,
}
impl JudicialCalendarQuery {
    pub fn new(
        limit: u32,
        after_id: Option<JudicialCalendarId>,
        status: JudicialCalendarStatusFilter,
        jurisdiction: Option<JudicialCalendarJurisdiction>,
        entity_code: Option<&str>,
    ) -> Result<Self, ApplicationError> {
        validate_limit(limit, 100)?;
        if entity_code.is_some_and(|v| {
            v.len() != 2 || !v.bytes().all(|b| b.is_ascii_digit()) || !("01"..="32").contains(&v)
        }) {
            return Err(ApplicationError::InvalidInput(
                "invalid calendar entity code".into(),
            ));
        }
        Ok(Self {
            limit,
            after_id,
            status,
            jurisdiction,
            entity_code: entity_code.map(str::to_owned),
        })
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn after_id(&self) -> Option<JudicialCalendarId> {
        self.after_id
    }
    pub const fn status(&self) -> JudicialCalendarStatusFilter {
        self.status
    }
    pub const fn jurisdiction(&self) -> Option<JudicialCalendarJurisdiction> {
        self.jurisdiction
    }
    pub fn entity_code(&self) -> Option<&str> {
        self.entity_code.as_deref()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudicialCalendarHistoryQuery {
    limit: u32,
    before_revision: Option<JudicialCalendarRevision>,
}
impl JudicialCalendarHistoryQuery {
    pub fn new(limit: u32, before_revision: Option<u32>) -> Result<Self, ApplicationError> {
        validate_limit(limit, 20)?;
        Ok(Self {
            limit,
            before_revision: before_revision
                .map(JudicialCalendarRevision::new)
                .transpose()?,
        })
    }
    pub const fn limit(self) -> u32 {
        self.limit
    }
    pub const fn before_revision(self) -> Option<JudicialCalendarRevision> {
        self.before_revision
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudicialCalendarDaysQuery {
    from: CivilDate,
    through: CivilDate,
}
impl JudicialCalendarDaysQuery {
    pub fn new(from: CivilDate, through: CivilDate) -> Result<Self, ApplicationError> {
        if !(1..=62).contains(&(through.days_since_epoch() - from.days_since_epoch() + 1)) {
            return Err(ApplicationError::InvalidInput(
                "calendar day range must contain 1 to 62 days".into(),
            ));
        }
        Ok(Self { from, through })
    }
    pub const fn from(self) -> CivilDate {
        self.from
    }
    pub const fn through(self) -> CivilDate {
        self.through
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarDays {
    pub calendar_id: JudicialCalendarId,
    pub revision: JudicialCalendarRevision,
    pub values_digest: domain::crypto::Sha256Digest,
    pub days: Vec<JudicialCalendarDay>,
}
fn validate_limit(limit: u32, max: u32) -> Result<(), ApplicationError> {
    if !(1..=max).contains(&limit) {
        return Err(ApplicationError::InvalidInput(
            "invalid calendar page limit".into(),
        ));
    }
    Ok(())
}
