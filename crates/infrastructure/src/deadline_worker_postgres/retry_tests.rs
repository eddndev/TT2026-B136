use super::classify;
use application::{
    deadline_profiles::DeadlineProfileError,
    deadline_worker::{DeadlineWorkerErrorCode as Code, DeadlineWorkerFailureKind as Kind},
    deadlines::DeadlineError,
    hearing_results::HearingResultError,
    hearings::HearingError,
    judicial_calendars::JudicialCalendarError,
    procedural_facts::ProceduralFactError,
    ApplicationError as A, PortFailureKind as Port,
};

#[test]
fn native_port_categories_remain_transient_even_during_job_authentication() {
    for verifying_job in [false, true] {
        for (kind, code) in [
            (Port::Busy, Code::LockUnavailable),
            (Port::Interrupted, Code::TransactionInterrupted),
            (Port::Unavailable, Code::DatabaseUnavailable),
        ] {
            let error = A::ClassifiedPort {
                kind,
                message: "opaque diagnostic".into(),
            };
            assert_eq!(classify(&error, verifying_job), (Kind::Transient, code));
        }
    }
}

#[test]
fn all_persisted_dependency_failures_are_integrity_errors_at_the_current_stage() {
    let errors = [
        A::Deadline(DeadlineError::StoredInconsistent("deadline".into())),
        A::DeadlineProfile(DeadlineProfileError::StoredInconsistent("profile".into())),
        A::JudicialCalendar(JudicialCalendarError::StoredInconsistent("calendar".into())),
        A::ProceduralFact(ProceduralFactError::StoredInconsistent("fact".into())),
        A::HearingResult(HearingResultError::StoredInconsistent("result".into())),
        A::Hearing(HearingError::StoredInconsistent("hearing".into())),
        A::StoredCaseAdministrationInconsistent("administration".into()),
        A::StoredCaseStageInconsistent("stage".into()),
        A::StoredParticipantInconsistent("participant".into()),
    ];
    for error in errors {
        assert_eq!(
            classify(&error, false),
            (Kind::Inconsistent, Code::InvalidStoredEvidence)
        );
        assert_eq!(
            classify(&error, true),
            (Kind::Inconsistent, Code::InvalidDurableJob)
        );
    }
}

#[test]
fn unclassified_failures_and_conflicts_are_not_inferred_from_diagnostics() {
    for error in [
        A::Port("55P03 lock unavailable".into()),
        A::Deadline(DeadlineError::OperationConflict),
        A::Deadline(DeadlineError::RevisionExhausted),
    ] {
        assert_eq!(
            classify(&error, false),
            (Kind::Transient, Code::ExecutionFailed)
        );
    }
}
