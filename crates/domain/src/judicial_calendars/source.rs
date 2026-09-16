use super::{text::required, CivilDate};
use crate::DomainError;
use uuid::Uuid;

pub struct JudicialCalendarSourceInput<'a> {
    pub id: Uuid,
    pub title: &'a str,
    pub issuer: &'a str,
    pub official_url: &'a str,
    pub published_on: Option<CivilDate>,
    pub consulted_on: CivilDate,
    pub locator: &'a str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarSource {
    id: Uuid,
    title: String,
    issuer: String,
    official_url: String,
    published_on: Option<CivilDate>,
    consulted_on: CivilDate,
    locator: String,
}
impl JudicialCalendarSource {
    pub fn new(input: JudicialCalendarSourceInput<'_>) -> Result<Self, DomainError> {
        if input
            .published_on
            .is_some_and(|day| day > input.consulted_on)
        {
            return Err(DomainError::InvalidJudicialCalendarValue(
                "source.published_on",
            ));
        }
        Ok(Self {
            id: input.id,
            title: required(input.title, 200, false, "source.title")?,
            issuer: required(input.issuer, 200, false, "source.issuer")?,
            official_url: public_url(input.official_url)?,
            published_on: input.published_on,
            consulted_on: input.consulted_on,
            locator: required(input.locator, 512, false, "source.locator")?,
        })
    }
    pub const fn id(&self) -> Uuid {
        self.id
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn issuer(&self) -> &str {
        &self.issuer
    }
    pub fn official_url(&self) -> &str {
        &self.official_url
    }
    pub const fn published_on(&self) -> Option<CivilDate> {
        self.published_on
    }
    pub const fn consulted_on(&self) -> CivilDate {
        self.consulted_on
    }
    pub fn locator(&self) -> &str {
        &self.locator
    }
}
fn public_url(input: &str) -> Result<String, DomainError> {
    let invalid = || DomainError::InvalidJudicialCalendarValue("source.official_url");
    if !input.is_ascii() || input.bytes().any(|b| b.is_ascii_control() || b == b'\\') {
        return Err(invalid());
    }
    let value = input.trim();
    if value.len() > 2048 || !value.starts_with("https://") {
        return Err(invalid());
    }
    let rest = &value[8..];
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = &rest[..host_end];
    let labels: Vec<_> = host.split('.').collect();
    if host.len() > 253
        || labels.len() < 2
        || labels.iter().any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.to_ascii_lowercase().starts_with("xn--")
                || !label.as_bytes()[0].is_ascii_alphanumeric()
                || !label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err(invalid());
    }
    let tld = labels.last().ok_or_else(invalid)?;
    if tld.len() < 2 || !tld.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err(invalid());
    }
    let tail = &rest.as_bytes()[host_end..];
    let mut index = 0;
    let mut fragments = 0;
    while index < tail.len() {
        let byte = tail[index];
        if byte == b'%' {
            if index + 2 >= tail.len()
                || !tail[index + 1].is_ascii_hexdigit()
                || !tail[index + 2].is_ascii_hexdigit()
            {
                return Err(invalid());
            }
            index += 3;
            continue;
        }
        if !byte.is_ascii_alphanumeric() && !b"-._~!$&'()*+,;=:@/?#".contains(&byte) {
            return Err(invalid());
        }
        if byte == b'#' {
            fragments += 1;
        }
        if fragments > 1 {
            return Err(invalid());
        }
        index += 1;
    }
    Ok(value.to_owned())
}
