use super::{invalid, read_expected, read_fields, reader::Reader, DeadlineProfileError};
use crate::deadline_profiles::{
    DeadlineProfileCondition, DeadlineProfileDefinitionInput, DeadlineProfileExample,
};
use domain::judicial_calendars::{
    JudicialCalendarValues, MAX_JUDICIAL_CALENDAR_CANONICAL_BYTES,
    MIN_JUDICIAL_CALENDAR_CANONICAL_BYTES,
};

pub(super) fn decode(
    reader: &mut Reader<'_>,
) -> Result<DeadlineProfileDefinitionInput, DeadlineProfileError> {
    let title = reader.label()?;
    let description = reader.fact_text()?;
    let scope = read_fields::scope(reader)?;
    let count = reader.count()?;
    let mut references = Vec::with_capacity(count);
    for _ in 0..count {
        references.push(read_fields::source(reader)?);
    }
    let trigger = reader.requirement()?;
    let template = read_fields::template(reader)?;
    let completion = read_fields::completion(reader)?;
    let count = reader.count()?;
    let mut conditions = Vec::with_capacity(count);
    for _ in 0..count {
        conditions.push(DeadlineProfileCondition {
            id: reader.uuid()?,
            statement: reader.fact_text()?,
            reference_ids: reader.reference_ids()?,
        });
    }
    let count = reader.count()?;
    let mut examples = Vec::with_capacity(count);
    for _ in 0..count {
        examples.push(DeadlineProfileExample {
            id: reader.uuid()?,
            anchor: reader.declared_time()?,
            ordered_quantity: reader.optional_quantity()?,
            calendar: calendar(reader)?,
            expected: read_expected::expected(reader)?,
            reference_ids: reader.reference_ids()?,
            locator: reader.label()?,
        });
    }
    Ok(DeadlineProfileDefinitionInput {
        title,
        description,
        scope,
        references,
        trigger,
        template,
        completion,
        conditions,
        examples,
    })
}

fn calendar(
    reader: &mut Reader<'_>,
) -> Result<Option<JudicialCalendarValues>, DeadlineProfileError> {
    if !reader.flag()? {
        return Ok(None);
    }
    let length = usize::try_from(reader.u32()?).map_err(invalid)?;
    if !(MIN_JUDICIAL_CALENDAR_CANONICAL_BYTES..=MAX_JUDICIAL_CALENDAR_CANONICAL_BYTES)
        .contains(&length)
    {
        return Err(invalid("embedded calendar length"));
    }
    JudicialCalendarValues::from_canonical_bytes(reader.take(length)?)
        .map(Some)
        .map_err(invalid)
}
