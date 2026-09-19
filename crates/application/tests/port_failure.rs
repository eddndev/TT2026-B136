use application::{ApplicationError, PortFailureKind};

#[test]
fn classified_ports_preserve_diagnostics_without_inferring_category_from_text() {
    for kind in [
        PortFailureKind::Unavailable,
        PortFailureKind::Busy,
        PortFailureKind::Interrupted,
    ] {
        let error = ApplicationError::ClassifiedPort {
            kind,
            message: "opaque diagnostic".into(),
        };
        assert_eq!(error.to_string(), "port failure: opaque diagnostic");
        let ApplicationError::ClassifiedPort {
            kind: actual,
            message,
        } = error
        else {
            panic!("a classified failure must retain its port category")
        };
        assert_eq!(actual, kind);
        assert_eq!(message, "opaque diagnostic");
    }
}

#[test]
fn legacy_ports_keep_their_existing_unclassified_contract() {
    let error = ApplicationError::Port("connection busy; 55P03".into());
    assert_eq!(error.to_string(), "port failure: connection busy; 55P03");
    assert!(matches!(error, ApplicationError::Port(_)));
}
