use super::{DeadlineDispatchBatch, DeadlineDispatchRequest};
use crate::ApplicationError;

/// Expand at most one page of one durable stream in an audited transaction.
///
/// The adapter acquires the existing audit lock before reading cursor, events
/// or current deadline dependencies. It inserts unique immutable jobs, advances
/// only the selected stream and appends its audit event atomically. Returned
/// progress reflects a committed state; an empty unchanged poll writes nothing.
///
/// Selection follows the current deadline revision and typed dependency scope,
/// including a notification's resolution parent. Closed cases and revoked
/// historical responsible accounts remain eligible. Retired deadlines are
/// terminal. The adapter does not reuse human authorization or fabricate users.
///
/// Event sequence gaps are valid. A partial page saves its last processed UUID;
/// an exhausted legacy sweep resets its cursor for future passes. Stable job
/// and operation identities survive reopen and retry after an uncertain reply.
/// The caller alternates streams and releases the lock after every page.
pub trait DeadlineDispatchStore: Send + Sync {
    fn dispatch(
        &self,
        request: DeadlineDispatchRequest,
    ) -> Result<DeadlineDispatchBatch, ApplicationError>;
}
