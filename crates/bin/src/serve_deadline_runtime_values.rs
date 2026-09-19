//! Opaque worker return values for scheduling tests.

use application::{
    deadline_dispatch::*,
    deadline_reevaluation::{
        DependencyFamily, PredecessorReceipt, SourceEventReference, TechnicalCause,
    },
    deadline_technical::{DeadlineReevaluationCommand, DeadlineReevaluationNoChange},
    deadline_worker::*,
    deadlines::{DeadlineId, DeadlineOperationId, DeadlineRevision},
};
use domain::{cases::CaseId, crypto::Sha256Digest, typed_participants::Uuid};
use time::{Duration, OffsetDateTime};

pub(super) fn batch(request: DeadlineDispatchRequest, selected: u32) -> DeadlineDispatchBatch {
    let mut progress = DeadlineDispatchProgress::default();
    let event = if selected > 0 && request.stream == DeadlineDispatchStream::Events {
        progress.event.active_sequence = Some(1);
        progress.event.after_deadline_id = Some(DeadlineId::from_uuid(Uuid::from_u128(2)));
        Some(SourceEventReference {
            sequence: 1,
            family: DependencyFamily::Profile,
            source_id: Uuid::from_u128(6),
            revision: 2,
            case_id: Some(CaseId::from_uuid(Uuid::from_u128(1))),
            hearing_id: None,
            operation_id: Uuid::from_u128(7),
        })
    } else {
        None
    };
    if selected > 0 && request.stream == DeadlineDispatchStream::LegacyBootstrap {
        progress.bootstrap_after_deadline_id = Some(DeadlineId::from_uuid(Uuid::from_u128(2)));
    }
    DeadlineDispatchBatch {
        stream: request.stream,
        event,
        selected,
        inserted: selected,
        completed_scan: selected == 0 && request.stream == DeadlineDispatchStream::LegacyBootstrap,
        progress,
    }
}

fn base() -> DeadlineWorkerBase {
    // The scheduling layer treats port evidence as opaque and never authenticates it.
    DeadlineWorkerBase {
        revision: DeadlineRevision::new(1).unwrap(),
        receipt: PredecessorReceipt {
            submission_digest: Sha256Digest::from_array([1; 32]),
            capture_digest: Sha256Digest::from_array([2; 32]),
        },
    }
}

pub(super) fn completed() -> DeadlineWorkerRun {
    let job_id = Uuid::from_u128(3);
    DeadlineWorkerRun::Completed(Box::new(DeadlineWorkerResult {
        job_id,
        case_id: CaseId::from_uuid(Uuid::from_u128(1)),
        deadline_id: DeadlineId::from_uuid(Uuid::from_u128(2)),
        command: DeadlineReevaluationCommand {
            operation_id: DeadlineOperationId::from_uuid(Uuid::from_u128(4)),
            cause: TechnicalCause::LegacyBootstrap {
                job_id,
                policy_version: 1,
            },
        },
        base: base(),
        outcome: DeadlineWorkerOutcome::NoChange {
            reason: DeadlineReevaluationNoChange::AlreadyInitialized,
            checked: None,
        },
        completed_at: OffsetDateTime::UNIX_EPOCH,
    }))
}

pub(super) fn deferred(inconsistent: bool) -> DeadlineWorkerRun {
    let (failure_kind, error_code, delay) = if inconsistent {
        (
            DeadlineWorkerFailureKind::Inconsistent,
            DeadlineWorkerErrorCode::InvalidStoredEvidence,
            3600,
        )
    } else {
        (
            DeadlineWorkerFailureKind::Transient,
            DeadlineWorkerErrorCode::DatabaseUnavailable,
            1,
        )
    };
    DeadlineWorkerRun::Deferred(DeadlineWorkerAttempt {
        attempt_id: Uuid::from_u128(5),
        job_id: Uuid::from_u128(3),
        attempt_number: 1,
        checked_base: Some(base()),
        failure_kind,
        error_code,
        failed_at: OffsetDateTime::UNIX_EPOCH,
        retry_at: OffsetDateTime::UNIX_EPOCH + Duration::seconds(delay),
    })
}
