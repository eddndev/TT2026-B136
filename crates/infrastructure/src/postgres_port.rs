//! Preserve neutral transport categories before database errors become messages.
use application::{ApplicationError, PortFailureKind};
use std::{error::Error as StdError, io::ErrorKind};

pub(crate) fn error(context: &str, error: postgres::Error) -> ApplicationError {
    let kind = classify(&error);
    let message = format!("{context}: {error}");
    match kind {
        Some(kind) => ApplicationError::ClassifiedPort { kind, message },
        None => ApplicationError::Port(message),
    }
}

fn classify(error: &postgres::Error) -> Option<PortFailureKind> {
    use PortFailureKind::{Busy, Interrupted, Unavailable};
    if let Some(state) = error.code() {
        return match state.code() {
            "55P03" => Some(Busy),
            "57014" | "40001" | "40P01" => Some(Interrupted),
            "57P01" | "57P02" | "57P03" => Some(Unavailable),
            state if state.starts_with("08") => Some(Unavailable),
            _ => None,
        };
    }
    if error.is_closed() {
        return Some(Unavailable);
    }
    let mut source = error.source();
    while let Some(cause) = source {
        if cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|cause| transport_kind(cause.kind()))
        {
            return Some(Unavailable);
        }
        source = cause.source();
    }
    None
}

fn transport_kind(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::ConnectionRefused
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::NotConnected
            | ErrorKind::BrokenPipe
            | ErrorKind::TimedOut
            | ErrorKind::UnexpectedEof
    )
}

#[cfg(test)]
mod tests {
    use super::{error, transport_kind};
    use application::{ApplicationError, PortFailureKind as Kind};
    use postgres::{Client, NoTls};
    use std::io::ErrorKind;

    struct Fixture(Client);

    impl Fixture {
        fn new() -> Option<Self> {
            let url = std::env::var("CASE_TEST_DATABASE_URL").ok()?;
            Some(Self(Client::connect(&url, NoTls).unwrap()))
        }

        fn raise(&mut self, sqlstate: &str) -> postgres::Error {
            assert_eq!(sqlstate.len(), 5);
            assert!(sqlstate.bytes().all(|byte| byte.is_ascii_alphanumeric()));
            self.0
                .batch_execute(&format!(
                    "DO $$ BEGIN RAISE EXCEPTION USING ERRCODE='{sqlstate}',
                    MESSAGE='55P03 57014 08006 connection closed deadlock timeout'; END $$"
                ))
                .unwrap_err()
        }
    }

    #[test]
    fn real_postgres_sqlstates_keep_neutral_failure_categories_and_display() {
        let Some(mut db) = Fixture::new() else { return };
        for (state, expected) in [
            ("55P03", Kind::Busy),
            ("57014", Kind::Interrupted),
            ("40001", Kind::Interrupted),
            ("40P01", Kind::Interrupted),
            ("08006", Kind::Unavailable),
            ("08P01", Kind::Unavailable),
            ("57P01", Kind::Unavailable),
            ("57P02", Kind::Unavailable),
            ("57P03", Kind::Unavailable),
        ] {
            let database = db.raise(state);
            assert_eq!(database.code().unwrap().code(), state);
            let message = format!("fixture database: {database}");
            let classified = error("fixture database", database);
            assert_eq!(classified.to_string(), format!("port failure: {message}"));
            let ApplicationError::ClassifiedPort {
                kind,
                message: actual,
            } = classified
            else {
                panic!("SQLSTATE {state} lost its failure category")
            };
            assert_eq!(kind, expected, "SQLSTATE {state}");
            assert_eq!(actual, message);
        }
    }

    #[test]
    fn unknown_postgres_sqlstates_remain_ports_despite_misleading_messages() {
        let Some(mut db) = Fixture::new() else { return };
        for state in ["23505", "23514", "42501", "22000", "P0001"] {
            let database = db.raise(state);
            assert_eq!(database.code().unwrap().code(), state);
            assert_eq!(
                database.as_db_error().unwrap().message(),
                "55P03 57014 08006 connection closed deadlock timeout"
            );
            let message = format!("fixture database: {database}");
            let unclassified = error("fixture database", database);
            assert_eq!(unclassified.to_string(), format!("port failure: {message}"));
            let ApplicationError::Port(actual) = unclassified else {
                panic!("SQLSTATE {state} was classified from an unrelated message")
            };
            assert_eq!(actual, message);
        }
    }

    #[test]
    fn native_transport_kinds_exclude_generic_and_codec_io_errors() {
        for kind in [
            ErrorKind::ConnectionRefused,
            ErrorKind::ConnectionReset,
            ErrorKind::ConnectionAborted,
            ErrorKind::NotConnected,
            ErrorKind::BrokenPipe,
            ErrorKind::TimedOut,
            ErrorKind::UnexpectedEof,
        ] {
            assert!(transport_kind(kind), "{kind:?}");
        }
        for kind in [
            ErrorKind::InvalidData,
            ErrorKind::InvalidInput,
            ErrorKind::Other,
        ] {
            assert!(!transport_kind(kind), "{kind:?}");
        }
    }
}
