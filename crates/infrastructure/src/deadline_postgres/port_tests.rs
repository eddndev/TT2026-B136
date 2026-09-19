use super::stored;
use application::{ApplicationError as A, PortFailureKind};

#[test]
fn captured_dependency_reads_preserve_typed_and_untyped_port_failures() {
    for kind in [
        PortFailureKind::Busy,
        PortFailureKind::Interrupted,
        PortFailureKind::Unavailable,
    ] {
        let error = stored(A::ClassifiedPort {
            kind,
            message: "opaque diagnostic".into(),
        });
        assert!(matches!(error, A::ClassifiedPort { kind: actual, .. } if actual == kind));
    }
    assert!(
        matches!(stored(A::Port("legacy diagnostic".into())), A::Port(message) if message == "legacy diagnostic")
    );
}

#[test]
fn absent_captured_dependencies_still_report_invalid_stored_evidence() {
    let error = stored(A::DeadlineProfile(
        application::deadline_profiles::DeadlineProfileError::NotFound,
    ));
    assert!(matches!(
        error,
        A::Deadline(application::deadlines::DeadlineError::StoredInconsistent(_))
    ));
}
