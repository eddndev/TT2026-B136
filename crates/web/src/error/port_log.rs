//! Name the neutral category of a port failure that HTTP reports opaquely.

use application::{ApplicationError, PortFailureKind};

/// Logs only the category: port messages may carry private storage detail.
pub(super) fn record(error: &ApplicationError) {
    if let Some(category) = category(error) {
        tracing::warn!(category, "request failed with an internal port error");
    }
}

pub(super) fn category(error: &ApplicationError) -> Option<&'static str> {
    Some(match error {
        ApplicationError::Port(_) => "port",
        ApplicationError::ClassifiedPort { kind, .. } => match kind {
            PortFailureKind::Busy => "busy",
            PortFailureKind::Interrupted => "interrupted",
            PortFailureKind::Unavailable => "unavailable",
        },
        _ => return None,
    })
}
