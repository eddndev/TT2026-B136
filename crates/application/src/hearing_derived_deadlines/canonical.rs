use super::*;
use crate::{
    deadline_evaluations::{
        deadline_evaluation_input_bytes, deadline_evaluation_record_bytes, DeadlineEvaluationRecord,
    },
    deadline_tracking::TrackingPolicy,
    deadlines::{evidence, hearing_result_projections, DeadlineChange},
};
use domain::crypto::DocumentHasher;

/// HRDL1 binds reviewed choices and exact projections, without a future capture time.
pub(super) fn review_digest(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    command: &HearingDerivedDeadlineCommand,
    material: &HearingDerivedDeadlineMaterial,
    evaluation: &ProfiledDeadlineEvaluation,
) -> Result<Sha256Digest, ApplicationError> {
    let (deadline, policies) = command.deadline.clone().into_parts();
    let DeadlineChange::Register { definition } = &deadline.change else {
        return Err(invalid("derived.deadline.action"));
    };
    let policies = policies.ok_or_else(|| invalid("derived.tracking"))?;
    let mut bytes = b"HRDL1".to_vec();
    bytes.extend_from_slice(actor.id.as_uuid().as_bytes());
    text(&mut bytes, &actor.email);
    text(&mut bytes, actor.role.as_str());
    let result = &material.result;
    bytes.extend_from_slice(result.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(&result.result_revision.get().to_be_bytes());
    bytes.extend_from_slice(result.values_digest.as_bytes());
    // Verified HRTX1 includes actor, case, operation, target and exact references.
    bytes.extend_from_slice(result.submission_digest.as_bytes());
    evidence::administration(&mut bytes, hasher, &result.observed_administration);
    hearing_result_projections(
        &mut bytes,
        &result.anchor,
        result.continuation.as_ref(),
        &result.attendees,
        result.support.as_ref(),
    );
    bytes.extend_from_slice(deadline.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(deadline.deadline_id.as_uuid().as_bytes());
    text(&mut bytes, definition.title.as_str());
    bytes.extend_from_slice(definition.profile.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&definition.profile.revision.get().to_be_bytes());
    bytes.extend_from_slice(definition.responsible.as_uuid().as_bytes());
    bytes.extend_from_slice(
        hasher
            .hash_bytes(&deadline_evaluation_input_bytes(&definition.input)?)
            .as_bytes(),
    );
    for profile in [&material.profile, &material.profile_head] {
        evidence::profile(&mut bytes, profile);
    }
    for calendar in [&material.calendar, &material.calendar_head] {
        bytes.push(u8::from(calendar.is_some()));
        if let Some(calendar) = calendar {
            evidence::calendar(&mut bytes, calendar);
        }
    }
    bytes.extend_from_slice(material.responsible.id.as_uuid().as_bytes());
    text(&mut bytes, &material.responsible.email);
    text(&mut bytes, material.responsible.role.as_str());
    for policy in [policies.profile, policies.source, policies.calendar] {
        bytes.push(match policy {
            TrackingPolicy::Follow => 0,
            TrackingPolicy::Fixed => 1,
            TrackingPolicy::Undetermined => 2,
        });
    }
    bytes.extend_from_slice(
        hasher
            .hash_bytes(&deadline_evaluation_record_bytes(
                &DeadlineEvaluationRecord::capture(evaluation),
            ))
            .as_bytes(),
    );
    Ok(hasher.hash_bytes(&bytes))
}

fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
