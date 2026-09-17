use super::{invalid, reader::Reader, DeadlineProfileError};
use crate::deadline_profiles::{
    DeadlineCivilCutoff, DeadlineCompletionPolicy, DeadlineProfileScope,
};
use domain::{
    cases::CaseId,
    deadline_arithmetic::{DayBasis, DayInclusion, FinalDayPolicy},
    deadline_profiles::{DeadlineRuleTemplate, OrderedDeadlineUnit},
    judicial_calendars::{
        JudicialCalendarJurisdiction, JudicialCalendarScope, JudicialCalendarScopeInput,
        JudicialCalendarSource, JudicialCalendarSourceInput,
    },
};
use time::{Time, UtcOffset};

pub(super) fn scope(reader: &mut Reader<'_>) -> Result<DeadlineProfileScope, DeadlineProfileError> {
    match reader.byte()? {
        0 => global_scope(reader).map(DeadlineProfileScope::Global),
        1 => Ok(DeadlineProfileScope::Case(CaseId::from_uuid(
            reader.uuid()?,
        ))),
        _ => Err(invalid("scope tag")),
    }
}
fn global_scope(reader: &mut Reader<'_>) -> Result<JudicialCalendarScope, DeadlineProfileError> {
    let title = reader.text(800)?;
    let jurisdiction = match reader.byte()? {
        0 => JudicialCalendarJurisdiction::Federal,
        1 => JudicialCalendarJurisdiction::Local,
        _ => return Err(invalid("jurisdiction tag")),
    };
    let count = reader.byte()?;
    if !(1..=32).contains(&count) {
        return Err(invalid("entity count"));
    }
    let mut codes = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let code = reader.byte()?;
        if !(1..=32).contains(&code) {
            return Err(invalid("entity code"));
        }
        codes.push(format!("{code:02}"));
    }
    let refs: Vec<_> = codes.iter().map(String::as_str).collect();
    JudicialCalendarScope::new(JudicialCalendarScopeInput {
        title,
        jurisdiction,
        entity_codes: &refs,
        authority: reader.text(800)?,
        organ: reader.text(800)?,
        territory: reader.text(800)?,
        use_description: reader.text(4000)?,
    })
    .map_err(invalid)
}
pub(super) fn source(
    reader: &mut Reader<'_>,
) -> Result<JudicialCalendarSource, DeadlineProfileError> {
    JudicialCalendarSource::new(JudicialCalendarSourceInput {
        id: reader.uuid()?,
        title: reader.text(800)?,
        issuer: reader.text(800)?,
        official_url: reader.text(2048)?,
        published_on: if reader.flag()? {
            Some(reader.date()?)
        } else {
            None
        },
        consulted_on: reader.date()?,
        locator: reader.text(2048)?,
    })
    .map_err(invalid)
}
fn final_day(reader: &mut Reader<'_>) -> Result<FinalDayPolicy, DeadlineProfileError> {
    Ok(if reader.flag()? {
        FinalDayPolicy::NextCountable
    } else {
        FinalDayPolicy::Preserve
    })
}
pub(super) fn template(
    reader: &mut Reader<'_>,
) -> Result<DeadlineRuleTemplate, DeadlineProfileError> {
    match reader.byte()? {
        0 => Ok(DeadlineRuleTemplate::Fixed(reader.rule()?)),
        1 => {
            let unit = match reader.byte()? {
                0 => OrderedDeadlineUnit::Days {
                    inclusion: if reader.flag()? {
                        DayInclusion::AfterAnchor
                    } else {
                        DayInclusion::OnAnchor
                    },
                    basis: if reader.flag()? {
                        DayBasis::CalendarCountable
                    } else {
                        DayBasis::Natural
                    },
                    final_day: final_day(reader)?,
                },
                1 => OrderedDeadlineUnit::CivilMonths {
                    final_day: final_day(reader)?,
                },
                2 => OrderedDeadlineUnit::ElapsedHours,
                _ => return Err(invalid("ordered unit")),
            };
            Ok(DeadlineRuleTemplate::Ordered {
                unit,
                maximum: reader.optional_quantity()?,
            })
        }
        _ => Err(invalid("template tag")),
    }
}
pub(super) fn completion(
    reader: &mut Reader<'_>,
) -> Result<DeadlineCompletionPolicy, DeadlineProfileError> {
    match reader.byte()? {
        0 => Ok(DeadlineCompletionPolicy::ArithmeticInstant),
        1 => Ok(DeadlineCompletionPolicy::CivilCandidateOnly),
        2 => {
            let time =
                Time::from_hms(reader.byte()?, reader.byte()?, reader.byte()?).map_err(invalid)?;
            let offset = UtcOffset::from_whole_seconds(reader.i32()?).map_err(invalid)?;
            DeadlineCivilCutoff::new(
                time,
                offset,
                reader.date()?,
                reader.date()?,
                reader.label()?,
                reader.uuid()?,
            )
            .map(DeadlineCompletionPolicy::CivilCutoff)
        }
        _ => Err(invalid("completion tag")),
    }
}
