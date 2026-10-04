use super::*;
use crate::{
    deadline_evaluations::{deadline_evaluation_record_bytes, DeadlineEvaluationRecord},
    deadline_inputs::{DeadlineInputMaterial, DeadlineSourceDetail},
    deadline_reevaluation::SourceEventReference,
    deadlines::*,
    hearing_results::HearingResultDetail,
};
use domain::crypto::DocumentHasher;

/// Calculate the final consequence against an actual first result capture.
/// Storage must resolve the existing event and commit result, deadline and origin
/// together after current authorization and exact-dependency revalidation.
/// This is confirmation logic, not a historical-read arithmetic replacement.
pub fn finalize_hearing_derived_deadline(
    hasher: &dyn DocumentHasher,
    draft: &HearingDerivedDeadlineDraft,
    result: HearingResultDetail,
    source_event: SourceEventReference,
) -> Result<HearingDerivedDeadlineCreation, ApplicationError> {
    super::recorded_source::validate(hasher, draft, &result, source_event)?;
    let case = result.snapshot.case_id;
    let (command, policies) = draft.command().deadline.clone().into_parts();
    let material = draft.material();
    let source = Some(DeadlineSourceDetail::HearingResult(Box::new(
        result.clone(),
    )));
    let preparation = DeadlinePreparation {
        case_id: case,
        deadline_id: command.deadline_id,
        administration: material.result.observed_administration.clone(),
        base: None,
        resolved: Some(DeadlineResolvedInputs {
            profile: material.profile.clone(),
            profile_head: material.profile_head.clone(),
            material: DeadlineInputMaterial {
                case_id: case,
                administration: material.result.observed_administration.clone(),
                source: source.clone(),
                source_head: source,
                calendar: material.calendar.clone(),
                calendar_head: material.calendar_head.clone(),
            },
            notification_parent_head: None,
        }),
        responsible: Some(material.responsible.clone()),
    };
    let author = DeadlineActorSnapshot::User {
        id: draft.actor().id,
        email: draft.actor().email.clone(),
    };
    let final_deadline = prepare_tracked_deadline_change(
        hasher,
        author.clone(),
        case,
        command.clone(),
        preparation,
        policies,
        None,
    )?;
    let expected = DeadlineEvaluationRecord::capture(draft.evaluation());
    if deadline_evaluation_record_bytes(&expected)
        != deadline_evaluation_record_bytes(&final_deadline.calculation().result)
    {
        return Err(DeadlineError::SubmissionMismatch.into());
    }
    let deadline = DeadlineDetail {
        id: command.deadline_id,
        case_id: case,
        revision: DeadlineRevision::initial(),
        definition: final_deadline.definition().clone(),
        calculation: final_deadline.calculation().clone(),
        tracking: final_deadline.tracking().cloned(),
        responsible: final_deadline.responsible().clone(),
        attention: final_deadline.attention().clone(),
        status: final_deadline.status(),
        reason: None,
        receipt: final_deadline.receipt(),
        recorded_at: result.snapshot.recorded_at,
        recorded_by: author,
    };
    deadline_receipt_matches(hasher, &deadline)?;
    let mut creation = HearingDerivedDeadlineCreation {
        draft: draft.clone(),
        result,
        deadline,
        source_event,
        capture_digest: Sha256Digest::from_array([0; 32]),
    };
    creation.capture_digest =
        hasher.hash_bytes(&hearing_derived_deadline_capture_bytes(&creation)?);
    Ok(creation)
}
