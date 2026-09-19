use super::{projection, values};
use crate::error::ApiError;
use application::procedural_facts::*;
use domain::{
    cases::CaseId,
    hearing_results::{DeclaredHearingResultPrecision, DeclaredHearingResultTime},
};
use serde_json::{json, Value};

pub(super) fn validate(
    case: CaseId,
    values: &ProceduralFactValues,
    sources: &FactSources,
) -> Result<(), ApiError> {
    // The application verifies hashes and persisted material. This boundary checks
    // exact selection and canonical shape before exposing a port response as JSON.
    fact_sources_bytes(sources).map_err(|_| ApiError::internal())?;
    let selection = FactSourceSelection::from_values(values);
    let resolved = &sources.resolved;
    if resolved.resolution.map(|s| s.reference) != selection.resolution()
        || resolved.resolution.is_some_and(|s| s.case_id != case)
        || resolved.participants.len() != selection.participants().len()
        || resolved.hearing_results.len() != selection.hearing_results().len()
        || sources.direct_supports.len() != selection.direct_supports().len()
        || resolved
            .participants
            .iter()
            .zip(selection.participants())
            .any(|(s, r)| s.case_id != case || s.reference != *r)
        || resolved
            .hearing_results
            .iter()
            .zip(selection.hearing_results())
            .any(|(s, r)| s.case_id != case || s.reference != *r)
        || sources
            .direct_supports
            .iter()
            .zip(selection.direct_supports())
            .any(|(s, r)| s.reference != r.reference() || s.digest != r.digest())
    {
        return Err(ApiError::internal());
    }
    Ok(())
}

pub(crate) fn project(sources: &FactSources) -> Result<Value, ApiError> {
    fact_sources_bytes(sources).map_err(|_| ApiError::internal())?;
    let resolution = match (&sources.resolved.resolution, &sources.views.resolution) {
        (Some(s), Some(v)) => Some(json!({"case_id":s.case_id,
            "id":s.reference.id.to_string(),"revision":s.reference.revision.get(),
            "values_digest":s.values_digest.to_hex(),"submission_digest":s.submission_digest.to_hex(),
            "status":projection::status(s.status),"class":values::class(&v.class),
            "issuer":values::label(&v.issuer),"issued_at":values::time(v.issued_at)?,"summary":v.summary.as_str()
        })),
        (None, None) => None,
        _ => return Err(ApiError::internal()),
    };
    let participants = sources.resolved.participants.iter().zip(&sources.views.participants)
        .map(|(s,v)| json!({"case_id":s.case_id,"id":s.reference.id.to_string(),
            "revision":s.reference.revision.get(),"values_digest":s.values_digest.to_hex(),
            "directory_status":s.status.as_str(),
            "subject":s.subject.map(|r|json!({"id":r.id.to_string(),"revision":r.revision.get(),"values_digest":r.values_digest.to_hex()})),
            "display_name":v.display_name,"procedural_role":v.procedural_role,"organization":v.organization,
            "kind":v.kind.map(|k|k.as_str())
        })).collect::<Vec<_>>();
    let hearings = sources.resolved.hearing_results.iter().zip(&sources.views.hearing_results)
        .map(|(s,v)| json!({"case_id":s.case_id,"hearing_id":s.reference.hearing_id.to_string(),
            "result_id":s.reference.result_id.to_string(),"revision":s.reference.revision.get(),
            "agreement_id":s.reference.agreement_id.map(|id|id.to_string()),
            "values_digest":s.values_digest.to_hex(),"submission_digest":s.submission_digest.to_hex(),
            "status":s.status.as_str(),"occurrence":v.occurrence.as_str(),"event_time":hearing_time(v.event_time),
            "summary":v.summary.as_str(),"agreement":v.agreement.as_ref().map(|a|json!({"id":a.id().to_string(),"text":a.text().as_str()}))
        })).collect::<Vec<_>>();
    let supports = sources.direct_supports.iter().map(|s|json!({
        "document_id":s.reference.id.to_string(),"version":s.reference.version.get(),"digest":s.digest.to_hex(),
        "name":s.name,"format":s.format.as_str(),"policy":s.policy.as_str()
    })).collect::<Vec<_>>();
    Ok(
        json!({"resolution":resolution,"participants":participants,"hearing_results":hearings,"direct_supports":supports}),
    )
}

fn hearing_time(value: DeclaredHearingResultTime) -> Value {
    let date = value.local_date();
    let mut result = json!({"precision":match value.precision() {
        DeclaredHearingResultPrecision::Date => "date", DeclaredHearingResultPrecision::Instant => "instant"
    },"year":date.year(),"month":u8::from(date.month()),"day":date.day(),
        "offset_seconds":value.offset().whole_seconds()});
    if let Some(instant) = value.instant_value() {
        result["hour"] = json!(instant.hour());
        result["minute"] = json!(instant.minute());
        result["second"] = json!(instant.second());
    }
    result
}
