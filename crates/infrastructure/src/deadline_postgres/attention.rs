use super::inconsistent;
use application::{deadlines::DeadlineAttention, ApplicationError};
use domain::{
    procedural_facts::{FactLabel, FactText},
    procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime},
};
use serde_json::{json, Value};

pub(super) fn encode(value: &DeadlineAttention) -> Value {
    match value {
        DeadlineAttention::Pending => json!({"status":"pending"}),
        DeadlineAttention::Recorded {
            occurred_at,
            statement,
            locator,
        } => json!({
            "status":"recorded", "occurred_at":declared(*occurred_at),
            "statement":statement.as_str(), "locator":locator.as_str(),
        }),
    }
}
pub(super) fn decode(value: &Value) -> Result<DeadlineAttention, ApplicationError> {
    let attention = match value["status"].as_str() {
        Some("pending") => DeadlineAttention::Pending,
        Some("recorded") => DeadlineAttention::Recorded {
            occurred_at: crate::procedural_fact_codec::declared_time(&value["occurred_at"])
                .map_err(inconsistent)?,
            statement: FactText::new(
                value["statement"]
                    .as_str()
                    .ok_or_else(|| inconsistent("attention statement absent"))?,
            )
            .map_err(inconsistent)?,
            locator: FactLabel::new(
                value["locator"]
                    .as_str()
                    .ok_or_else(|| inconsistent("attention locator absent"))?,
            )
            .map_err(inconsistent)?,
        },
        _ => return Err(inconsistent("unknown attention status")),
    };
    if encode(&attention) != *value {
        return Err(inconsistent("noncanonical attention projection"));
    }
    Ok(attention)
}
fn declared(value: DeclaredProceduralTime) -> Value {
    let precision = match value.precision() {
        DeclaredProceduralPrecision::Unknown => return json!({"precision":"unknown"}),
        DeclaredProceduralPrecision::Date => "date",
        DeclaredProceduralPrecision::Minute => "minute",
        DeclaredProceduralPrecision::Second => "second",
    };
    let date = value
        .local_date()
        .expect("known declaration has a date")
        .date();
    let mut result = json!({"precision":precision,"year":date.year(),"month":date.month() as u8,
        "day":date.day(),"offset_seconds":value.offset().map(|offset|offset.whole_seconds())});
    if let Some(hour) = value.local_hour() {
        result["hour"] = json!(hour);
        result["minute"] = json!(value.local_minute().expect("hour declaration has a minute"));
    }
    if let Some(second) = value.local_second() {
        result["second"] = json!(second);
    }
    result
}
