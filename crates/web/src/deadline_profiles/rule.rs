use super::{
    definition::{invalid, quantity, reference},
    object,
};
use crate::error::ApiError;
use application::deadline_profiles::*;
use domain::{
    deadline_arithmetic::*,
    deadline_profiles::{DeadlineRuleTemplate, OrderedDeadlineUnit},
    deadline_triggers::*,
};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Trigger {
    SourceField { field: Field },
    Qualified { purpose: Purpose, family: Family },
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Field {
    ResolutionIssuedAt,
    NotificationPracticedAt,
    NotificationReceivedAt,
    NotificationStatedEffectAt,
    HearingSessionEventTime,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Purpose {
    HearingEnd,
    OrderedPeriodStart,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Family {
    Resolution,
    Notification,
    HearingResult,
}
impl Trigger {
    pub(super) fn validate(self) -> TriggerRequirement {
        match self {
            Self::SourceField { field } => TriggerRequirement::SourceField(match field {
                Field::ResolutionIssuedAt => TriggerField::ResolutionIssuedAt,
                Field::NotificationPracticedAt => TriggerField::NotificationPracticedAt,
                Field::NotificationReceivedAt => TriggerField::NotificationReceivedAt,
                Field::NotificationStatedEffectAt => TriggerField::NotificationStatedEffectAt,
                Field::HearingSessionEventTime => TriggerField::HearingSessionEventTime,
            }),
            Self::Qualified { purpose, family } => TriggerRequirement::Qualified {
                purpose: match purpose {
                    Purpose::HearingEnd => QualifiedTriggerPurpose::HearingEnd,
                    Purpose::OrderedPeriodStart => QualifiedTriggerPurpose::OrderedPeriodStart,
                },
                family: match family {
                    Family::Resolution => TriggerFamily::Resolution,
                    Family::Notification => TriggerFamily::Notification,
                    Family::HearingResult => TriggerFamily::HearingResult,
                },
            },
        }
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Template {
    Fixed {
        #[serde(deserialize_with = "object::deserialize")]
        rule: Rule,
    },
    Ordered {
        #[serde(deserialize_with = "object::deserialize")]
        unit: Unit,
        maximum: Option<u32>,
    },
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Rule {
    Days {
        quantity: u32,
        inclusion: Inclusion,
        basis: Basis,
        final_day: FinalDay,
    },
    CivilMonths {
        quantity: u32,
        final_day: FinalDay,
    },
    ElapsedHours {
        quantity: u32,
    },
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Unit {
    Days {
        inclusion: Inclusion,
        basis: Basis,
        final_day: FinalDay,
    },
    CivilMonths {
        final_day: FinalDay,
    },
    ElapsedHours {},
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Inclusion {
    OnAnchor,
    AfterAnchor,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Basis {
    Natural,
    CalendarCountable,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum FinalDay {
    Preserve,
    NextCountable,
}
impl Inclusion {
    fn value(self) -> DayInclusion {
        match self {
            Self::OnAnchor => DayInclusion::OnAnchor,
            Self::AfterAnchor => DayInclusion::AfterAnchor,
        }
    }
}
impl Basis {
    fn value(self) -> DayBasis {
        match self {
            Self::Natural => DayBasis::Natural,
            Self::CalendarCountable => DayBasis::CalendarCountable,
        }
    }
}
impl FinalDay {
    fn value(self) -> FinalDayPolicy {
        match self {
            Self::Preserve => FinalDayPolicy::Preserve,
            Self::NextCountable => FinalDayPolicy::NextCountable,
        }
    }
}
impl Template {
    pub(super) fn validate(self) -> Result<DeadlineRuleTemplate, ApiError> {
        Ok(match self {
            Self::Fixed { rule } => DeadlineRuleTemplate::Fixed(match rule {
                Rule::Days {
                    quantity: q,
                    inclusion,
                    basis,
                    final_day,
                } => ArithmeticRule::Days {
                    quantity: quantity(q)?,
                    inclusion: inclusion.value(),
                    basis: basis.value(),
                    final_day: final_day.value(),
                },
                Rule::CivilMonths {
                    quantity: q,
                    final_day,
                } => ArithmeticRule::CivilMonths {
                    quantity: quantity(q)?,
                    final_day: final_day.value(),
                },
                Rule::ElapsedHours { quantity: q } => ArithmeticRule::ElapsedHours {
                    quantity: quantity(q)?,
                },
            }),
            Self::Ordered { unit, maximum } => DeadlineRuleTemplate::Ordered {
                unit: match unit {
                    Unit::Days {
                        inclusion,
                        basis,
                        final_day,
                    } => OrderedDeadlineUnit::Days {
                        inclusion: inclusion.value(),
                        basis: basis.value(),
                        final_day: final_day.value(),
                    },
                    Unit::CivilMonths { final_day } => OrderedDeadlineUnit::CivilMonths {
                        final_day: final_day.value(),
                    },
                    Unit::ElapsedHours {} => OrderedDeadlineUnit::ElapsedHours,
                },
                maximum: maximum.map(quantity).transpose()?,
            },
        })
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Completion {
    ArithmeticInstant {},
    CivilCandidateOnly {},
    CivilCutoff {
        time: String,
        offset_seconds: i32,
        from: String,
        through: String,
        channel: String,
        reference_id: String,
    },
}
impl Completion {
    pub(super) fn validate(self) -> Result<DeadlineCompletionPolicy, ApiError> {
        Ok(match self {
            Self::ArithmeticInstant {} => DeadlineCompletionPolicy::ArithmeticInstant,
            Self::CivilCandidateOnly {} => DeadlineCompletionPolicy::CivilCandidateOnly,
            Self::CivilCutoff {
                time,
                offset_seconds,
                from,
                through,
                channel,
                reference_id,
            } => {
                if !time.is_ascii()
                    || time.len() != 8
                    || &time[2..3] != ":"
                    || &time[5..6] != ":"
                    || !time
                        .bytes()
                        .enumerate()
                        .all(|(i, b)| i == 2 || i == 5 || b.is_ascii_digit())
                {
                    return Err(invalid());
                }
                let t = time::Time::from_hms(
                    time[..2].parse().map_err(|_| invalid())?,
                    time[3..5].parse().map_err(|_| invalid())?,
                    time[6..].parse().map_err(|_| invalid())?,
                )
                .map_err(|_| invalid())?;
                DeadlineCompletionPolicy::CivilCutoff(
                    DeadlineCivilCutoff::new(
                        t,
                        time::UtcOffset::from_whole_seconds(offset_seconds)
                            .map_err(|_| invalid())?,
                        from.parse().map_err(|_| invalid())?,
                        through.parse().map_err(|_| invalid())?,
                        domain::procedural_facts::FactLabel::new(&channel)
                            .map_err(application::ApplicationError::from)?,
                        reference(&reference_id)?,
                    )
                    .map_err(application::ApplicationError::from)?,
                )
            }
        })
    }
}
