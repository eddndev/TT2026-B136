//! Pure technical preparation over verified deadline history and dependency heads.
//! The repository must authenticate the durable event and service, then recheck
//! the base and all heads when committing the revision or a no-change outcome.
mod model;
mod policy;
mod validation;
pub use model::*;

use crate::{
    deadline_evaluations::{
        evaluate_profiled_deadline, DeadlineEvaluationError, DeadlineEvaluationRecord,
    },
    deadline_inputs::DeadlineCalendarRef,
    deadline_observations::{build_deadline_observations, build_legacy_deadline_observations},
    deadline_reevaluation::{encode_observations, PredecessorReceipt, TechnicalCause},
    deadline_tracking::{DeadlineReviewState, TrackingPolicies, TrackingPolicy},
    deadlines::*,
    ApplicationError,
};
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    procedural_facts::FactText,
};

type Result<T> = std::result::Result<T, ApplicationError>;

/// Build a new capture without granting human acceptance or changing retained
/// legal qualifications. No-change outcomes still require a transactional base
/// check before the repository can mark their durable job complete.
pub fn prepare_technical_deadline_change(
    hasher: &dyn DocumentHasher,
    base: &DeadlineDetail,
    command: DeadlineReevaluationCommand,
    inputs: DeadlineReevaluationInputs,
) -> Result<DeadlineReevaluationOutcome> {
    deadline_receipt_matches(hasher, base)?;
    crate::deadline_reevaluation::validate_technical_cause(base.case_id, command.cause)
        .map_err(inconsistent)?;
    if base.status == DeadlineStatus::Retired {
        return Ok(DeadlineReevaluationOutcome::NoChange(
            DeadlineReevaluationNoChange::Retired,
        ));
    }
    if matches!(command.cause, TechnicalCause::LegacyBootstrap { .. })
        && base.review_state() != DeadlineReviewState::LegacyUndeclared
    {
        return Ok(DeadlineReevaluationOutcome::NoChange(
            DeadlineReevaluationNoChange::AlreadyInitialized,
        ));
    }
    let old_observations = match &base.tracking {
        Some(tracking) => tracking.observations.clone(),
        None => build_legacy_deadline_observations(hasher, base)?,
    };
    validation::selected(hasher, base, &inputs)?;
    let observations = build_deadline_observations(
        hasher,
        base.case_id,
        &inputs.profile_head,
        &inputs.material,
        inputs.notification_parent_head.as_ref(),
    )?;
    validation::advance(&old_observations, &observations)?;
    if let Some(reason) =
        validation::cause(&old_observations, &observations, command.cause, &inputs)?
    {
        return Ok(DeadlineReevaluationOutcome::NoChange(reason));
    }
    if command.operation_id == base.receipt.operation_id {
        return Err(DeadlineError::OperationConflict.into());
    }
    let policies = base.tracking.as_ref().map_or(
        TrackingPolicies {
            profile: TrackingPolicy::Undetermined,
            source: TrackingPolicy::Undetermined,
            calendar: TrackingPolicy::Undetermined,
        },
        |tracking| tracking.policies,
    );
    let (review, recalculate) =
        policy::review(base, &inputs, policies, &old_observations, &observations)?;
    let tracking = DeadlineTrackingCapture {
        policies,
        review,
        observations,
        administration: inputs.material.administration.clone(),
    };
    let mut definition = base.definition.clone();
    let mut calculation = base.calculation.clone();
    if recalculate {
        let calendar = inputs
            .material
            .calendar_head
            .as_ref()
            .ok_or_else(|| inconsistent("followed calendar head is absent"))?;
        definition.input.calendar = Some(DeadlineCalendarRef {
            id: calendar.id,
            revision: calendar.revision,
        });
        calculation.material.calendar = Some(calendar.clone());
        calculation.material.calendar_head = Some(calendar.clone());
        let evaluated = evaluate_profiled_deadline(
            hasher,
            &calculation.profile.definition,
            &definition.input,
            &calculation.material,
        )
        .map_err(|error| match error {
            DeadlineEvaluationError::Inputs(error) => error,
            DeadlineEvaluationError::Invalid(field) => DeadlineError::Invalid(field).into(),
        })?;
        calculation.result = DeadlineEvaluationRecord::capture(&evaluated);
    }
    let zero = Sha256Digest::from_array([0; 32]);
    let observations_digest =
        hasher.hash_bytes(&encode_observations(&tracking.observations).map_err(inconsistent)?);
    let mut prepared = PreparedDeadlineReevaluation {
        base: base.clone(),
        inputs,
        definition,
        calculation,
        tracking,
        revision: base.revision.next()?,
        reason: FactText::new(match command.cause {
            TechnicalCause::LegacyBootstrap { .. } => {
                "Legacy tracking requires explicit human policies"
            }
            TechnicalCause::SourceEvent { .. } => {
                "Dependency changes observed by the deadline reevaluator"
            }
        })
        .map_err(inconsistent)?,
        receipt: DeadlineReceipt {
            version: DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
                observations_digest,
                predecessor: Some(PredecessorReceipt {
                    submission_digest: base.receipt.submission_digest,
                    capture_digest: base.receipt.capture_digest,
                }),
                cause: Some(command.cause),
            }),
            operation_id: command.operation_id,
            action: DeadlineAction::Reevaluate,
            expected_revision: base.revision.get(),
            review_digest: zero,
            capture_digest: zero,
            submission_digest: zero,
        },
    };
    // Storage supplies the recording time. The canonical contracts and transition
    // checks below do not use that time to recalculate or qualify legal inputs.
    let record = prepared.record(base.recorded_at);
    prepared.receipt.review_digest = hasher.hash_bytes(&deadline_review_bytes(hasher, &record)?);
    prepared.receipt.capture_digest = hasher.hash_bytes(&deadline_capture_bytes(hasher, &record)?);
    prepared.receipt.submission_digest = hasher.hash_bytes(&deadline_record_submission_bytes(
        &prepared.record(base.recorded_at),
    )?);
    deadline_successor_matches(hasher, base, &prepared.record(base.recorded_at))?;
    Ok(DeadlineReevaluationOutcome::Revision(Box::new(prepared)))
}

fn inconsistent(message: impl std::fmt::Display) -> ApplicationError {
    DeadlineError::StoredInconsistent(message.to_string()).into()
}
