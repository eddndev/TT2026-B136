use super::{codec, preferences, stored, subject};
use application::{alerts::*, ApplicationError};
use domain::{alerts::AlertWindow, identity::UserId};
use serde_json::{json, Value};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub(super) struct Plan {
    pub key: String,
    pub occurrence: Uuid,
    pub kind: AlertKind,
    pub trigger: OffsetDateTime,
    pub channels: AlertChannels,
    pub superseded: bool,
}

pub(super) fn state(
    previous: &Value,
    current: &subject::Verified,
    now: OffsetDateTime,
) -> Result<Value, ApplicationError> {
    let old_due = codec::optional_time(&previous["last_due"])?;
    let due = current.activity_at;
    let mut result = previous.clone();
    result["snapshot"] = subject::snapshot(current);
    result["last_due"] = codec::optional(due.or(old_due));
    let overdue = current.attention_pending && due.is_some_and(|d| now >= d);
    for (field, active) in [
        ("review_episode", current.review),
        ("overdue_episode", overdue),
    ] {
        if !active {
            result[field] = Value::Null;
        } else if previous[field].is_null() || (field == "overdue_episode" && old_due != due) {
            result[field] = json!(Uuid::new_v4());
        }
    }
    if let (Some(before), Some(after)) = (old_due, due) {
        if before != after {
            result["changed_episode"] = if [before, after]
                .iter()
                .any(|at| *at >= now && (*at - now) <= Duration::hours(48))
            {
                json!([
                    Uuid::new_v4(),
                    codec::instant(before),
                    codec::instant(after)
                ])
            } else {
                Value::Null
            };
        }
    }
    if let Some(ended) = current.ended {
        let _ = ended;
        result["review_episode"] = Value::Null;
        result["overdue_episode"] = Value::Null;
        result["changed_episode"] = Value::Null;
    }
    Ok(result)
}

pub(super) fn plans(
    current: &subject::Verified,
    state: &Value,
    prefs: &AlertPreferenceValues,
    now: OffsetDateTime,
) -> Result<Vec<Plan>, ApplicationError> {
    let mut result = Vec::new();
    let (family, id) = codec::subject_key(current.subject);
    let prefix = format!("v1:{family}:{id}");
    let upcoming = if family == 0 {
        &prefs.hearing_upcoming
    } else {
        &prefs.deadline_upcoming
    };
    if let Some(due) = current.activity_at {
        if now < due {
            let closest = upcoming
                .anticipations
                .hours()
                .iter()
                .filter(|lead| {
                    AlertWindow::upcoming(due, **lead).is_ok_and(|window| window.contains(now))
                })
                .map(|lead| lead.get())
                .min();
            for lead in upcoming.anticipations.hours() {
                let window = AlertWindow::upcoming(due, *lead).map_err(stored)?;
                let key = format!(
                    "{prefix}:upcoming:{}:{}:{}",
                    due.unix_timestamp(),
                    due.nanosecond(),
                    lead.get()
                );
                result.push(Plan {
                    key,
                    occurrence: Uuid::new_v4(),
                    kind: AlertKind::Upcoming {
                        lead_hours: *lead,
                        activity_at: due,
                    },
                    trigger: window.starts_at(),
                    channels: upcoming.channels,
                    superseded: window.contains(now) && closest != Some(lead.get()),
                });
            }
        }
    }
    for (field, label, channels) in [
        ("review_episode", "review", prefs.review_required),
        ("overdue_episode", "overdue", prefs.overdue_unattended),
    ] {
        if !state[field].is_null() {
            let occurrence = codec::uuid(&state[field])?;
            let kind = if field == "review_episode" {
                AlertKind::ReviewRequired
            } else {
                AlertKind::OverdueUnattended {
                    due_at: current
                        .activity_at
                        .ok_or_else(|| stored("overdue episode has no current instant"))?,
                }
            };
            result.push(Plan {
                key: format!("{prefix}:{label}:{occurrence}"),
                occurrence,
                kind,
                trigger: if let AlertKind::OverdueUnattended { due_at } = kind {
                    due_at
                } else {
                    now
                },
                channels,
                superseded: false,
            });
        }
    }
    if !state["changed_episode"].is_null() {
        let episode = &state["changed_episode"];
        let occurrence = codec::uuid(&episode[0])?;
        let before = codec::time(&episode[1])?;
        let after = codec::time(&episode[2])?;
        if current.activity_at == Some(after) {
            result.push(Plan {
                key: format!("{prefix}:changed:{occurrence}"),
                occurrence,
                kind: AlertKind::DueChangedSoon {
                    previous_due_at: before,
                    current_due_at: after,
                },
                trigger: now,
                channels: prefs.due_changed_soon,
                superseded: false,
            });
        }
    }
    Ok(result)
}

pub(super) fn encode(plan: &Plan, current: &subject::Verified, recipient: UserId) -> Value {
    json!({"key":plan.key,"occurrence":plan.occurrence,"subject":codec::subject(current.subject),"recipient":recipient.as_uuid(),
        "kind":codec::kind(plan.kind),"origin":codec::origin(current.origin),"trigger":codec::instant(plan.trigger),
        "channels":preferences::channels(plan.channels),"subject_title":current.subject_title,"case_title":current.case_title,"case_reference":current.case_reference})
}
pub(super) fn channels(
    kind: AlertKind,
    prefs: &AlertPreferenceValues,
    subject: AlertSubject,
) -> AlertChannels {
    match kind {
        AlertKind::Upcoming { lead_hours, .. } => {
            let family = if matches!(subject, AlertSubject::Hearing { .. }) {
                &prefs.hearing_upcoming
            } else {
                &prefs.deadline_upcoming
            };
            if family.anticipations.hours().contains(&lead_hours) {
                family.channels
            } else {
                AlertChannels {
                    internal: false,
                    email: false,
                }
            }
        }
        AlertKind::ReviewRequired => prefs.review_required,
        AlertKind::OverdueUnattended { .. } => prefs.overdue_unattended,
        AlertKind::DueChangedSoon { .. } => prefs.due_changed_soon,
    }
}
