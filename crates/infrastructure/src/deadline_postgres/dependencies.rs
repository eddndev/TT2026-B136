use super::{header::counter, inconsistent};
use application::{
    deadline_evaluations::DeadlineEvaluationInput, deadline_inputs::*, deadlines::DeadlineDetail,
    ApplicationError,
};
use domain::{
    deadline_triggers::TriggerSourceRef,
    hearing_results::HearingResultRevision,
    judicial_calendars::JudicialCalendarRevision,
    procedural_facts::{FactDeclaration, FactRevision},
};
use postgres::Row;
use uuid::Uuid;

/// Typed references duplicated from canonical inputs for foreign keys and dependency queries.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Columns {
    pub source_kind: Option<String>,
    pub source_id: Option<Uuid>,
    pub source_revision: Option<i64>,
    pub source_head_revision: Option<i64>,
    pub source_hearing_id: Option<Uuid>,
    pub source_parent_resolution_id: Option<Uuid>,
    pub source_parent_resolution_revision: Option<i64>,
    pub source_head_parent_resolution_revision: Option<i64>,
    pub calendar_id: Option<Uuid>,
    pub calendar_revision: Option<i64>,
    pub calendar_head_revision: Option<i64>,
}
impl Columns {
    pub fn capture(detail: &DeadlineDetail) -> Result<Self, ApplicationError> {
        from_input(
            &detail.definition.input,
            &DeadlineInputHeads::capture(&detail.calculation.material),
        )
    }
}
fn from_input(
    input: &DeadlineEvaluationInput,
    heads: &DeadlineInputHeads,
) -> Result<Columns, ApplicationError> {
    let mut result = Columns {
        source_kind: None,
        source_id: None,
        source_revision: None,
        source_head_revision: None,
        source_hearing_id: None,
        source_parent_resolution_id: None,
        source_parent_resolution_revision: None,
        source_head_parent_resolution_revision: None,
        calendar_id: None,
        calendar_revision: None,
        calendar_head_revision: None,
    };
    match (&input.selection.source, heads.source) {
        (FactDeclaration::Unknown(_), None) => {}
        (
            FactDeclaration::Known(TriggerSourceRef::Resolution(selected)),
            Some(TriggerSourceRef::Resolution(head)),
        ) if selected.id == head.id => {
            result.source_kind = Some("resolution".into());
            result.source_id = Some(selected.id.as_uuid());
            result.source_revision = Some(i64::from(selected.revision.get()));
            result.source_head_revision = Some(i64::from(head.revision.get()));
        }
        (
            FactDeclaration::Known(TriggerSourceRef::Notification {
                id,
                revision,
                resolution,
            }),
            Some(TriggerSourceRef::Notification {
                id: head_id,
                revision: head_revision,
                resolution: head_parent,
            }),
        ) if *id == head_id && resolution.id == head_parent.id => {
            result.source_kind = Some("notification".into());
            result.source_id = Some(id.as_uuid());
            result.source_revision = Some(i64::from(revision.get()));
            result.source_head_revision = Some(i64::from(head_revision.get()));
            result.source_parent_resolution_id = Some(resolution.id.as_uuid());
            result.source_parent_resolution_revision = Some(i64::from(resolution.revision.get()));
            result.source_head_parent_resolution_revision =
                Some(i64::from(head_parent.revision.get()));
        }
        (
            FactDeclaration::Known(TriggerSourceRef::HearingResult(selected)),
            Some(TriggerSourceRef::HearingResult(head)),
        ) if selected.result_id == head.result_id
            && selected.hearing_id == head.hearing_id
            && head.agreement_id.is_none() =>
        {
            result.source_kind = Some("hearing_result".into());
            result.source_id = Some(selected.result_id.as_uuid());
            result.source_revision = Some(i64::from(selected.revision.get()));
            result.source_head_revision = Some(i64::from(head.revision.get()));
            result.source_hearing_id = Some(selected.hearing_id.as_uuid());
        }
        _ => return Err(inconsistent("deadline source and head identities differ")),
    }
    match (input.calendar, heads.calendar) {
        (None, None) => {}
        (Some(selected), Some(head)) if selected.id == head.id => {
            result.calendar_id = Some(selected.id.as_uuid());
            result.calendar_revision = Some(i64::from(selected.revision.get()));
            result.calendar_head_revision = Some(i64::from(head.revision.get()));
        }
        _ => return Err(inconsistent("deadline calendar and head identities differ")),
    }
    Ok(result)
}
fn required_counter(raw: Option<i64>) -> Result<u32, ApplicationError> {
    counter(raw.ok_or_else(|| inconsistent("deadline observed revision absent"))?)
}
pub(super) fn read(
    row: &Row,
    input: &DeadlineEvaluationInput,
) -> Result<DeadlineInputHeads, ApplicationError> {
    let columns = Columns {
        source_kind: row.try_get("source_kind").map_err(inconsistent)?,
        source_id: row.try_get("source_id").map_err(inconsistent)?,
        source_revision: row.try_get("source_revision").map_err(inconsistent)?,
        source_head_revision: row.try_get("source_head_revision").map_err(inconsistent)?,
        source_hearing_id: row.try_get("source_hearing_id").map_err(inconsistent)?,
        source_parent_resolution_id: row
            .try_get("source_parent_resolution_id")
            .map_err(inconsistent)?,
        source_parent_resolution_revision: row
            .try_get("source_parent_resolution_revision")
            .map_err(inconsistent)?,
        source_head_parent_resolution_revision: row
            .try_get("source_head_parent_resolution_revision")
            .map_err(inconsistent)?,
        calendar_id: row.try_get("calendar_id").map_err(inconsistent)?,
        calendar_revision: row.try_get("calendar_revision").map_err(inconsistent)?,
        calendar_head_revision: row
            .try_get("calendar_head_revision")
            .map_err(inconsistent)?,
    };
    let source = match input.selection.source {
        FactDeclaration::Unknown(_) => None,
        FactDeclaration::Known(mut selected) => {
            let revision = required_counter(columns.source_head_revision)?;
            match &mut selected {
                TriggerSourceRef::Resolution(reference) => {
                    reference.revision = FactRevision::new(revision).map_err(inconsistent)?
                }
                TriggerSourceRef::Notification {
                    revision: number,
                    resolution,
                    ..
                } => {
                    *number = FactRevision::new(revision).map_err(inconsistent)?;
                    resolution.revision = FactRevision::new(required_counter(
                        columns.source_head_parent_resolution_revision,
                    )?)
                    .map_err(inconsistent)?;
                }
                TriggerSourceRef::HearingResult(reference) => {
                    reference.revision =
                        HearingResultRevision::new(revision).map_err(inconsistent)?;
                    reference.agreement_id = None;
                }
            }
            Some(selected)
        }
    };
    let calendar = input
        .calendar
        .map(|mut selected| {
            selected.revision =
                JudicialCalendarRevision::new(required_counter(columns.calendar_head_revision)?)
                    .map_err(inconsistent)?;
            Ok::<_, ApplicationError>(selected)
        })
        .transpose()?;
    let heads = DeadlineInputHeads { source, calendar };
    if columns != from_input(input, &heads)? {
        return Err(inconsistent(
            "deadline dependency columns differ from canonical inputs",
        ));
    }
    Ok(heads)
}

pub(super) fn projection(input: &DeadlineEvaluationInput) -> serde_json::Value {
    use serde_json::json;
    let mut result = json!({"case_id":input.selection.case_id.to_string(),
        "source_kind":null,"source_id":null,"source_revision":null,"source_hearing_id":null,
        "source_parent_resolution_id":null,"source_parent_resolution_revision":null,"source_agreement_id":null,
        "calendar_id":input.calendar.map(|r|r.id.to_string()),"calendar_revision":input.calendar.map(|r|r.revision.get())});
    if let FactDeclaration::Known(source) = input.selection.source {
        match source {
            TriggerSourceRef::Resolution(r) => {
                result["source_kind"] = json!("resolution");
                result["source_id"] = json!(r.id.to_string());
                result["source_revision"] = json!(r.revision.get());
            }
            TriggerSourceRef::Notification {
                id,
                revision,
                resolution,
            } => {
                result["source_kind"] = json!("notification");
                result["source_id"] = json!(id.to_string());
                result["source_revision"] = json!(revision.get());
                result["source_parent_resolution_id"] = json!(resolution.id.to_string());
                result["source_parent_resolution_revision"] = json!(resolution.revision.get());
            }
            TriggerSourceRef::HearingResult(r) => {
                result["source_kind"] = json!("hearing_result");
                result["source_id"] = json!(r.result_id.to_string());
                result["source_revision"] = json!(r.revision.get());
                result["source_hearing_id"] = json!(r.hearing_id.to_string());
                result["source_agreement_id"] = json!(r.agreement_id.map(|id| id.to_string()));
            }
        }
    }
    result
}
