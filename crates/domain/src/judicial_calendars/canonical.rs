use super::{CivilDate, JudicialCalendarRule, JudicialCalendarValues};

impl JudicialCalendarValues {
    /// JCAL1 binds normalized civil-day values without publication metadata.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"JCAL1".to_vec();
        let scope = self.scope();
        text(&mut bytes, scope.title());
        bytes.push(scope.jurisdiction().tag());
        bytes.push(scope.entity_codes().len() as u8);
        for code in scope.entity_codes() {
            bytes.push((code.as_bytes()[0] - b'0') * 10 + code.as_bytes()[1] - b'0');
        }
        text(&mut bytes, scope.authority());
        text(&mut bytes, scope.organ());
        text(&mut bytes, scope.territory());
        text(&mut bytes, scope.use_description());
        date(&mut bytes, self.coverage().from());
        date(&mut bytes, self.coverage().through());
        bytes.push(self.sources().len() as u8);
        for source in self.sources() {
            bytes.extend_from_slice(source.id().as_bytes());
            text(&mut bytes, source.title());
            text(&mut bytes, source.issuer());
            text(&mut bytes, source.official_url());
            bytes.push(u8::from(source.published_on().is_some()));
            if let Some(value) = source.published_on() {
                date(&mut bytes, value);
            }
            date(&mut bytes, source.consulted_on());
            text(&mut bytes, source.locator());
        }
        for weekday in self.weekly_pattern() {
            bytes.push(weekday.weekday());
            rule(&mut bytes, weekday.rule());
        }
        bytes.push(self.exceptions().len() as u8);
        for exception in self.exceptions() {
            bytes.extend_from_slice(exception.id().as_bytes());
            date(&mut bytes, exception.from());
            date(&mut bytes, exception.through());
            rule(&mut bytes, exception.rule());
        }
        bytes
    }
}
fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
fn date(bytes: &mut Vec<u8>, value: CivilDate) {
    bytes.extend_from_slice(&value.days_since_epoch().to_be_bytes());
}
fn rule(bytes: &mut Vec<u8>, value: &JudicialCalendarRule) {
    bytes.push(value.classification().tag());
    bytes.push(value.source_ids().len() as u8);
    for id in value.source_ids() {
        bytes.extend_from_slice(id.as_bytes());
    }
    text(bytes, value.explanation());
}
