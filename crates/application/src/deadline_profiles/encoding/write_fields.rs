use super::write::{optional_quantity, text};
use crate::{
    deadline_inputs::encoding::write as input,
    deadline_profiles::{DeadlineCompletionPolicy, DeadlineProfileScope},
};
use domain::{
    deadline_arithmetic::{DayBasis, DayInclusion, FinalDayPolicy},
    deadline_profiles::{DeadlineRuleTemplate, OrderedDeadlineUnit},
    judicial_calendars::JudicialCalendarSource,
};

pub(super) fn scope(bytes: &mut Vec<u8>, scope: &DeadlineProfileScope) {
    match scope {
        DeadlineProfileScope::Global(scope) => {
            bytes.push(0);
            text(bytes, scope.title());
            bytes.push(scope.jurisdiction().tag());
            bytes.push(scope.entity_codes().len() as u8);
            for code in scope.entity_codes() {
                // Scope constructors guarantee two ASCII decimal digits in 01..32.
                let digits = code.as_bytes();
                bytes.push((digits[0] - b'0') * 10 + digits[1] - b'0');
            }
            text(bytes, scope.authority());
            text(bytes, scope.organ());
            text(bytes, scope.territory());
            text(bytes, scope.use_description());
        }
        DeadlineProfileScope::Case(case_id) => {
            bytes.push(1);
            bytes.extend_from_slice(case_id.as_uuid().as_bytes());
        }
    }
}
pub(super) fn source(bytes: &mut Vec<u8>, source: &JudicialCalendarSource) {
    bytes.extend_from_slice(source.id().as_bytes());
    text(bytes, source.title());
    text(bytes, source.issuer());
    text(bytes, source.official_url());
    bytes.push(u8::from(source.published_on().is_some()));
    if let Some(date) = source.published_on() {
        bytes.extend_from_slice(&date.days_since_epoch().to_be_bytes());
    }
    bytes.extend_from_slice(&source.consulted_on().days_since_epoch().to_be_bytes());
    text(bytes, source.locator());
}
fn final_day(policy: FinalDayPolicy) -> u8 {
    match policy {
        FinalDayPolicy::Preserve => 0,
        FinalDayPolicy::NextCountable => 1,
    }
}
pub(super) fn template(bytes: &mut Vec<u8>, template: DeadlineRuleTemplate) {
    match template {
        DeadlineRuleTemplate::Fixed(rule) => {
            bytes.push(0);
            input::rule(bytes, rule);
        }
        DeadlineRuleTemplate::Ordered { unit, maximum } => {
            bytes.push(1);
            match unit {
                OrderedDeadlineUnit::Days {
                    inclusion,
                    basis,
                    final_day: policy,
                } => {
                    bytes.push(0);
                    bytes.push(match inclusion {
                        DayInclusion::OnAnchor => 0,
                        DayInclusion::AfterAnchor => 1,
                    });
                    bytes.push(match basis {
                        DayBasis::Natural => 0,
                        DayBasis::CalendarCountable => 1,
                    });
                    bytes.push(final_day(policy));
                }
                OrderedDeadlineUnit::CivilMonths { final_day: policy } => {
                    bytes.extend_from_slice(&[1, final_day(policy)]);
                }
                OrderedDeadlineUnit::ElapsedHours => bytes.push(2),
            }
            optional_quantity(bytes, maximum);
        }
    }
}
pub(super) fn completion(bytes: &mut Vec<u8>, completion: &DeadlineCompletionPolicy) {
    match completion {
        DeadlineCompletionPolicy::ArithmeticInstant => bytes.push(0),
        DeadlineCompletionPolicy::CivilCandidateOnly => bytes.push(1),
        DeadlineCompletionPolicy::CivilCutoff(cutoff) => {
            let time = cutoff.time();
            bytes.extend_from_slice(&[2, time.hour(), time.minute(), time.second()]);
            bytes.extend_from_slice(&cutoff.offset().whole_seconds().to_be_bytes());
            bytes.extend_from_slice(&cutoff.from().days_since_epoch().to_be_bytes());
            bytes.extend_from_slice(&cutoff.through().days_since_epoch().to_be_bytes());
            text(bytes, cutoff.channel().as_str());
            bytes.extend_from_slice(cutoff.reference_id().as_bytes());
        }
    }
}
