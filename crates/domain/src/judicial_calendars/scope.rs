use super::{text::required, JudicialCalendarJurisdiction};
use crate::DomainError;

pub struct JudicialCalendarScopeInput<'a> {
    pub title: &'a str,
    pub jurisdiction: JudicialCalendarJurisdiction,
    pub entity_codes: &'a [&'a str],
    pub authority: &'a str,
    pub organ: &'a str,
    pub territory: &'a str,
    pub use_description: &'a str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarScope {
    title: String,
    jurisdiction: JudicialCalendarJurisdiction,
    entity_codes: Vec<String>,
    authority: String,
    organ: String,
    territory: String,
    use_description: String,
}
impl JudicialCalendarScope {
    pub fn new(input: JudicialCalendarScopeInput<'_>) -> Result<Self, DomainError> {
        let mut codes: Vec<String> = input.entity_codes.iter().map(|v| (*v).to_owned()).collect();
        if !(1..=32).contains(&codes.len())
            || codes.iter().any(|v| {
                v.len() != 2
                    || !v.bytes().all(|b| b.is_ascii_digit())
                    || v.as_str() < "01"
                    || v.as_str() > "32"
            })
        {
            return Err(DomainError::InvalidJudicialCalendarValue(
                "scope.entity_codes",
            ));
        }
        codes.sort_unstable();
        if codes.windows(2).any(|w| w[0] == w[1]) {
            return Err(DomainError::InvalidJudicialCalendarValue(
                "scope.entity_codes",
            ));
        }
        Ok(Self {
            title: required(input.title, 200, false, "scope.title")?,
            jurisdiction: input.jurisdiction,
            entity_codes: codes,
            authority: required(input.authority, 200, false, "scope.authority")?,
            organ: required(input.organ, 200, false, "scope.organ")?,
            territory: required(input.territory, 200, false, "scope.territory")?,
            use_description: required(input.use_description, 1000, true, "scope.use_description")?,
        })
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub const fn jurisdiction(&self) -> JudicialCalendarJurisdiction {
        self.jurisdiction
    }
    pub fn entity_codes(&self) -> &[String] {
        &self.entity_codes
    }
    pub fn authority(&self) -> &str {
        &self.authority
    }
    pub fn organ(&self) -> &str {
        &self.organ
    }
    pub fn territory(&self) -> &str {
        &self.territory
    }
    pub fn use_description(&self) -> &str {
        &self.use_description
    }
}
