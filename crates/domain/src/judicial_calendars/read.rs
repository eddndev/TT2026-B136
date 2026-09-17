use super::*;
use crate::DomainError;
use uuid::Uuid;

impl JudicialCalendarValues {
    /// Strict JCAL1 decode: normalized and ordered bytes are the only stored form.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, DomainError> {
        if !(MIN_JUDICIAL_CALENDAR_CANONICAL_BYTES..=MAX_JUDICIAL_CALENDAR_CANONICAL_BYTES)
            .contains(&bytes.len())
        {
            return Err(invalid());
        }
        let mut read = Reader { bytes, at: 0 };
        if read.take(5)? != b"JCAL1" {
            return Err(invalid());
        }
        let scope = read.scope()?;
        let coverage = JudicialCalendarCoverage::new(read.date()?, read.date()?)?;
        let count = read.count(MAX_JUDICIAL_CALENDAR_SOURCES)?;
        let mut sources = Vec::with_capacity(count);
        for _ in 0..count {
            sources.push(read.source()?);
        }
        let mut weekly = Vec::with_capacity(7);
        for _ in 0..7 {
            weekly.push(JudicialCalendarWeekdayRule::new(
                read.byte()?,
                read.rule()?,
            )?);
        }
        let count = read.count(MAX_JUDICIAL_CALENDAR_EXCEPTIONS)?;
        let mut exceptions = Vec::with_capacity(count);
        for _ in 0..count {
            exceptions.push(JudicialCalendarException::new(
                read.uuid()?,
                read.date()?,
                read.date()?,
                read.rule()?,
            )?);
        }
        if read.at != bytes.len() {
            return Err(invalid());
        }
        let value = Self::new(scope, coverage, sources, weekly, exceptions)?;
        if value.canonical_bytes() != bytes {
            return Err(invalid());
        }
        Ok(value)
    }
}
fn invalid() -> DomainError {
    DomainError::InvalidJudicialCalendarValue("canonical")
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], DomainError> {
        let until = self.at.checked_add(length).ok_or_else(invalid)?;
        let value = self.bytes.get(self.at..until).ok_or_else(invalid)?;
        self.at = until;
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8, DomainError> {
        Ok(self.take(1)?[0])
    }
    fn count(&mut self, max: usize) -> Result<usize, DomainError> {
        let count = usize::from(self.byte()?);
        if count > max {
            return Err(invalid());
        }
        Ok(count)
    }
    fn text(&mut self, max_bytes: usize) -> Result<&'a str, DomainError> {
        let length = u32::from_be_bytes(self.take(4)?.try_into().map_err(|_| invalid())?) as usize;
        if length > max_bytes {
            return Err(invalid());
        }
        std::str::from_utf8(self.take(length)?).map_err(|_| invalid())
    }
    fn date(&mut self) -> Result<CivilDate, DomainError> {
        let days = i32::from_be_bytes(self.take(4)?.try_into().map_err(|_| invalid())?);
        CivilDate::from_days_since_epoch(days)
    }
    fn uuid(&mut self) -> Result<Uuid, DomainError> {
        Uuid::from_slice(self.take(16)?).map_err(|_| invalid())
    }
    fn scope(&mut self) -> Result<JudicialCalendarScope, DomainError> {
        let title = self.text(800)?;
        let jurisdiction = JudicialCalendarJurisdiction::from_tag(self.byte()?)?;
        let count = self.count(32)?;
        let mut codes = Vec::with_capacity(count);
        for _ in 0..count {
            codes.push(format!("{:02}", self.byte()?));
        }
        let refs: Vec<_> = codes.iter().map(String::as_str).collect();
        JudicialCalendarScope::new(JudicialCalendarScopeInput {
            title,
            jurisdiction,
            entity_codes: &refs,
            authority: self.text(800)?,
            organ: self.text(800)?,
            territory: self.text(800)?,
            use_description: self.text(4000)?,
        })
    }
    fn source(&mut self) -> Result<JudicialCalendarSource, DomainError> {
        let id = self.uuid()?;
        let title = self.text(800)?;
        let issuer = self.text(800)?;
        let official_url = self.text(2048)?;
        let published_on = match self.byte()? {
            0 => None,
            1 => Some(self.date()?),
            _ => return Err(invalid()),
        };
        let consulted_on = self.date()?;
        let locator = self.text(2048)?;
        JudicialCalendarSource::new(JudicialCalendarSourceInput {
            id,
            title,
            issuer,
            official_url,
            published_on,
            consulted_on,
            locator,
        })
    }
    fn rule(&mut self) -> Result<JudicialCalendarRule, DomainError> {
        let classification = JudicialCalendarClassification::from_tag(self.byte()?)?;
        let count = self.count(MAX_JUDICIAL_CALENDAR_SOURCES)?;
        let mut ids = Vec::with_capacity(count);
        for _ in 0..count {
            ids.push(self.uuid()?);
        }
        JudicialCalendarRule::new(classification, ids, self.text(1024)?)
    }
}
