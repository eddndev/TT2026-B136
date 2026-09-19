use super::{input_projection as input, result};
use crate::error::ApiError;
use application::{
    cases::CurrentCaseAdministration,
    deadline_inputs::*,
    deadlines::*,
    judicial_calendars::JudicialCalendarDetail,
    procedural_facts::{FactStatus, ProceduralFactSnapshot},
};
use domain::{
    cases::CaseId,
    deadline_triggers::TriggerSourceRef,
    procedural_facts::{FactDeclaration, FactHearingRef, FactResolutionRef},
};
use serde_json::{json, Value};

pub(super) fn reference(v: &DeadlineSourceDetail) -> TriggerSourceRef {
    match v {
        DeadlineSourceDetail::Fact(v) => match &v.snapshot {
            ProceduralFactSnapshot::Resolution(v) => {
                TriggerSourceRef::Resolution(FactResolutionRef {
                    id: v.root.id(),
                    revision: v.metadata.revision,
                })
            }
            ProceduralFactSnapshot::Notification(v) => TriggerSourceRef::Notification {
                id: v.root.id(),
                revision: v.metadata.revision,
                resolution: v.values.resolution(),
            },
        },
        DeadlineSourceDetail::HearingResult(v) => TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: v.snapshot.hearing_id,
            result_id: v.snapshot.id,
            revision: v.snapshot.revision,
            agreement_id: None,
        }),
    }
}
fn source(
    v: &DeadlineSourceDetail,
    case: CaseId,
    selection: Option<TriggerSourceRef>,
) -> Result<Value, ApiError> {
    let mut reference = reference(v);
    let (actual_case, values, submission, status, sources_digest, href) = match v {
        DeadlineSourceDetail::Fact(v) => {
            let meta = v.snapshot.metadata();
            let href = match &v.snapshot {
                ProceduralFactSnapshot::Resolution(v) => format!(
                    "/api/v1/cases/{case}/resolutions/{}/revisions/{}",
                    v.root.id(),
                    meta.revision.get()
                ),
                ProceduralFactSnapshot::Notification(v) => format!(
                    "/api/v1/cases/{case}/resolutions/{}/notifications/{}/revisions/{}",
                    v.root.resolution_id(),
                    v.root.id(),
                    meta.revision.get()
                ),
            };
            (
                v.snapshot.case_id(),
                meta.values_digest,
                meta.receipt.submission_digest,
                match meta.status {
                    FactStatus::Recorded => "recorded",
                    FactStatus::Withdrawn => "withdrawn",
                },
                Some(meta.receipt.sources_digest),
                href,
            )
        }
        DeadlineSourceDetail::HearingResult(v) => {
            let s = &v.snapshot;
            if let Some(TriggerSourceRef::HearingResult(selected)) = selection {
                if selected
                    .agreement_id
                    .is_some_and(|id| !s.values.agreements().iter().any(|v| v.id() == id))
                {
                    return Err(ApiError::internal());
                }
                if let TriggerSourceRef::HearingResult(ref mut reference) = reference {
                    reference.agreement_id = selected.agreement_id;
                }
            }
            (
                s.case_id,
                s.values_digest,
                s.receipt.submission_digest,
                s.status.as_str(),
                None,
                format!(
                    "/api/v1/cases/{case}/hearings/{}/results/{}/revisions/{}",
                    s.hearing_id,
                    s.id,
                    s.revision.get()
                ),
            )
        }
    };
    if actual_case != case || selection.is_some_and(|selected| selected != reference) {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"case_id":case.to_string(),"reference":input::source(&reference),"values_digest":values.to_hex(),"sources_digest":sources_digest.map(|v|v.to_hex()),"submission_digest":submission.to_hex(),"status":status,"href":href}),
    )
}
fn same_root(exact: &DeadlineSourceDetail, head: &DeadlineSourceDetail) -> bool {
    match (exact, head) {
        (DeadlineSourceDetail::Fact(a), DeadlineSourceDetail::Fact(b)) => {
            a.snapshot.target() == b.snapshot.target()
                && b.snapshot.metadata().revision >= a.snapshot.metadata().revision
                && (b.snapshot.metadata().revision != a.snapshot.metadata().revision || a == b)
        }
        (DeadlineSourceDetail::HearingResult(a), DeadlineSourceDetail::HearingResult(b)) => {
            a.snapshot.hearing_id == b.snapshot.hearing_id
                && a.snapshot.id == b.snapshot.id
                && b.snapshot.revision >= a.snapshot.revision
                && (b.snapshot.revision != a.snapshot.revision || a == b)
        }
        _ => false,
    }
}
fn calendar(v: &JudicialCalendarDetail) -> Value {
    json!({"id":v.id.to_string(),"revision":v.revision.get(),"values_digest":v.values_digest.to_hex(),"submission_digest":v.receipt.submission_digest.to_hex(),"status":v.status.as_str(),"title":v.values.scope().title(),"href":format!("/api/v1/judicial-calendars/{}/revisions/{}",v.id,v.revision.get())})
}
pub(super) fn material(
    v: &DeadlineInputMaterial,
    definition: &DeadlineDefinition,
    case: CaseId,
) -> Result<Value, ApiError> {
    if v.case_id != case || definition.input.selection.case_id != case {
        return Err(ApiError::internal());
    }
    let (source, head) = match (
        &definition.input.selection.source,
        &v.source,
        &v.source_head,
    ) {
        (FactDeclaration::Unknown(_), None, None) => (Value::Null, Value::Null),
        (FactDeclaration::Known(selected), Some(exact), Some(head)) if same_root(exact, head) => (
            source(exact, case, Some(*selected))?,
            source(head, case, None)?,
        ),
        _ => return Err(ApiError::internal()),
    };
    let (calendar, calendar_head) = match (definition.input.calendar, &v.calendar, &v.calendar_head)
    {
        (None, None, None) => (Value::Null, Value::Null),
        (Some(selected), Some(exact), Some(head))
            if selected.id == exact.id
                && selected.revision == exact.revision
                && head.id == exact.id
                && head.revision >= exact.revision
                && head.values.scope() == exact.values.scope()
                && (head.revision != exact.revision || head == exact) =>
        {
            (calendar(exact), calendar(head))
        }
        _ => return Err(ApiError::internal()),
    };
    Ok(
        json!({"case_id":case.to_string(),"administration":administration(&v.administration,case)?,"source":source,"source_head":head,"calendar":calendar,"calendar_head":calendar_head}),
    )
}
pub(super) fn administration(
    v: &CurrentCaseAdministration,
    case: CaseId,
) -> Result<Value, ApiError> {
    let values = v.values();
    let mut out = json!({"title":values.metadata().title(),"reference":values.metadata().reference(),"status":values.status().as_str()});
    match v {
        CurrentCaseAdministration::Unrevised(_) => out["kind"] = json!("unrevised"),
        CurrentCaseAdministration::Recorded(v) => {
            if v.case_id != case {
                return Err(ApiError::internal());
            }
            out["kind"] = json!("recorded");
            out["case_id"] = json!(case.to_string());
            out["revision"] = json!(v.revision.get());
            out["values_digest"] = json!(v.values_digest.to_hex());
            out["changed_at"] = result::instant(v.changed_at);
            out["changed_by"] = json!({"id":v.changed_by.id,"email":v.changed_by.email});
        }
    }
    Ok(out)
}
