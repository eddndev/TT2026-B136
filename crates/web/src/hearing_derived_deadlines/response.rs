use crate::{deadline_profiles, deadlines, error::ApiError, hearing_results, judicial_calendars};
use application::{
    deadline_evaluations::DeadlineEvaluationRecord,
    deadline_profiles::DeadlineProfileCollection,
    deadlines::{DeadlineChange, DeadlineRevision},
    hearing_derived_deadlines::*,
    hearing_results::HearingResultRevision,
    judicial_calendars::JudicialCalendarDetail,
};
use domain::{cases::CaseId, crypto::Sha256Digest};
use serde_json::{json, Value};

fn command(value: &HearingDerivedDeadlineCommand, case: CaseId) -> Result<Value, ApiError> {
    let (deadline, policies) = value.deadline.clone().into_parts();
    let mut deadline = deadlines::command_projection(&deadline)?;
    deadline["change"]["tracking"] =
        deadlines::policies_projection(&policies.ok_or_else(ApiError::internal)?);
    Ok(
        json!({"case_id":case,"result":hearing_results::command_projection(&value.result)?,
        "deadline":deadline}),
    )
}

pub(super) fn review(
    value: HearingDerivedDeadlineReview,
    case: CaseId,
    expected: &HearingDerivedDeadlineCommand,
) -> Result<Value, ApiError> {
    match value {
        HearingDerivedDeadlineReview::Ready(draft) => ready(*draft, case, expected),
        HearingDerivedDeadlineReview::Replay(value) => {
            Ok(json!({"state":"replay","record":record(*value,case,expected,None)?}))
        }
    }
}

fn ready(
    draft: HearingDerivedDeadlineDraft,
    case: CaseId,
    expected: &HearingDerivedDeadlineCommand,
) -> Result<Value, ApiError> {
    let expected_json = command(expected, case)?;
    if command(draft.command(), case)? != expected_json || draft.result().case_id != case {
        return Err(ApiError::internal());
    }
    let (value, policies) = expected.deadline.clone().into_parts();
    let DeadlineChange::Register { definition } = &value.change else {
        return Err(ApiError::internal());
    };
    let material = draft.material();
    let p = &material.profile;
    let head = &material.profile_head;
    let profile = deadline_profiles::exact_projection(
        p.clone(),
        DeadlineProfileCollection::ForCase(case),
        definition.profile.id,
        Some(definition.profile.revision),
    )?;
    let profile_head = deadline_profiles::exact_projection(
        head.clone(),
        DeadlineProfileCollection::ForCase(case),
        p.id,
        Some(head.revision),
    )?;
    Ok(json!({"state":"ready","command":expected_json,
        "result":hearing_results::draft_projection(material.result.clone(),case,&expected.result)?,
        "deadline":{
            "definition":deadlines::definition_projection(definition)?,
            "tracking":deadlines::policies_projection(&policies.ok_or_else(ApiError::internal)?),
            "responsible":deadlines::responsible_projection(&material.responsible)?,
            "profile":profile,"profile_head":profile_head,
            "calendar":calendar(material.calendar.as_ref())?,
            "calendar_head":calendar(material.calendar_head.as_ref())?,
            "result":deadlines::result_projection(&DeadlineEvaluationRecord::capture(draft.evaluation()))?
        },"review_digest":draft.review_digest().to_hex()}))
}

fn calendar(value: Option<&JudicialCalendarDetail>) -> Result<Option<Value>, ApiError> {
    value
        .map(|v| judicial_calendars::exact_projection(v.clone(), v.id, Some(v.revision)))
        .transpose()
}

pub(super) fn record(
    record: HearingDerivedDeadlineRecord,
    case: CaseId,
    expected: &HearingDerivedDeadlineCommand,
    expected_digest: Option<Sha256Digest>,
) -> Result<Value, ApiError> {
    // The opaque application record has already verified both canonical captures.
    // Here bind that original evidence to this HTTP scope and exact instruction.
    let e = record.evidence();
    let expected_json = command(expected, case)?;
    if e.result.snapshot.case_id != case
        || command(&e.command, case)? != expected_json
        || expected_digest.is_some_and(|value| value != e.review_digest)
    {
        return Err(ApiError::internal());
    }
    let result = hearing_results::exact_projection(
        e.result.clone(),
        case,
        expected.result.hearing_id,
        expected.result.result_id,
        Some(HearingResultRevision::initial()),
    )?;
    let (deadline, _) = expected.deadline.clone().into_parts();
    deadlines::validate_submission(
        &e.deadline,
        &expected.deadline,
        e.deadline.receipt.submission_digest,
    )?;
    let projected_deadline = deadlines::exact_projection(
        e.deadline.clone(),
        case,
        deadline.deadline_id,
        Some(DeadlineRevision::initial()),
    )?;
    let origin = json!({"case_id":case,"hearing_id":expected.result.hearing_id.to_string(),
        "result_id":expected.result.result_id.to_string(),"result_revision":1,
        "result_operation_id":expected.result.operation_id.to_string(),
        "deadline_id":deadline.deadline_id.to_string(),"deadline_revision":1,
        "deadline_operation_id":deadline.operation_id.to_string(),
        "recorded_by":{"id":e.actor.id,"email":e.actor.email,"role":e.actor.role.as_str()},
        "review_digest":e.review_digest.to_hex(),"capture_digest":e.capture_digest.to_hex(),
        "source_event":deadlines::source_event_projection(e.source_event,case)?});
    Ok(
        json!({"case_id":case,"command":expected_json,"result":result,"deadline":projected_deadline,
        "origin":origin,"review_digest":e.review_digest.to_hex(),"capture_digest":e.capture_digest.to_hex()}),
    )
}
