//! Owner-only cancellation of restored recovery capabilities with an audit receipt.

mod context;
mod mutation;
mod receipt;

use application::ApplicationError;
use domain::crypto::Sha256Digest;
use uuid::Uuid;

/// Exact audit predecessor expected by an administrative restoration operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PasswordResetRestoreHead {
    /// Zero-based sequence of the latest event before invalidation.
    pub sequence: u64,
    /// Chain digest at that sequence.
    pub chain: Sha256Digest,
}

/// Database identity and stable operation identity retained across uncertain retries.
#[derive(Clone, Debug)]
pub struct PasswordResetRestoreRequest {
    /// Fresh non-nil UUID for this restoration, reused only to reconcile its outcome.
    pub operation_id: Uuid,
    /// Exact database name of the restored snapshot.
    pub expected_database: String,
    /// Actual application schema, independent of any search-path prefix.
    pub expected_schema: String,
    /// Expected original audit head; None means the empty chain.
    pub expected_head: Option<PasswordResetRestoreHead>,
}

/// Receipt of one committed invalidation, including an exact reconciled retry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PasswordResetRestoreResult {
    /// True only when this invocation committed a new event.
    pub applied: bool,
    /// Number of pending capabilities cancelled by the recorded operation.
    pub invalidated: u64,
    /// Sequence of that operation's audit event, including on a later retry.
    pub audit_sequence: u64,
    /// Chain digest of that same event.
    pub audit_head: Sha256Digest,
}

/// Invalidates every restored pending capability using the administrative owner.
///
/// This does not migrate, grant privileges, change users, or invalidate Redis
/// sessions. The caller must stop writers and keep recovery admission closed
/// through the complete SQL and Redis restoration procedure. Preserve the request
/// after any uncertain result; retrying it returns the exact recorded receipt.
///
/// # Errors
/// Rejects a non-owner, altered catalog or authority, inconsistent inventory,
/// unexpected database/schema/audit head, reused operation identity, or invalid
/// server time. A storage error at commit can have an uncertain outcome.
pub fn invalidate_restored_password_resets(
    admin_url: &str,
    request: PasswordResetRestoreRequest,
) -> Result<PasswordResetRestoreResult, ApplicationError> {
    if request.operation_id.is_nil()
        || request.expected_database.is_empty()
        || request.expected_schema.is_empty()
        || request
            .expected_head
            .is_some_and(|head| head.sequence > i64::MAX as u64)
    {
        return Err(rejected());
    }
    // This private connection owns the session lock and releases it on every exit.
    let mut client = crate::postgres::connect_runtime(admin_url).map_err(|_| storage())?;
    context::pin_search_path(&mut client, &request.expected_schema)?;
    let target = context::resolve(&mut client, &request)?;
    context::lock(&mut client, &target)?;
    let mut transaction =
        crate::audit_postgres::begin_audited(&mut client).map_err(|_| storage())?;
    context::validate(&mut transaction, &request, &target)?;
    let events = receipt::verified_events(&mut transaction)?;
    let previous = receipt::existing(&events, &request)?;
    if previous.is_none() && receipt::head(events.last()) != request.expected_head {
        return Err(rejected());
    }
    let pending = mutation::lock_pending(&mut transaction)?;
    if let Some(result) = previous {
        if pending != 0 {
            return Err(rejected());
        }
        transaction.rollback().map_err(|_| storage())?;
        return Ok(result);
    }
    let at = mutation::cancel(&mut transaction, pending)?;
    let result = receipt::append(&mut transaction, request.operation_id, pending, at)?;
    transaction.commit().map_err(|_| storage())?;
    Ok(result)
}

fn rejected() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "password reset restoration target, authority, inventory, or receipt is invalid".into(),
    )
}

fn storage() -> ApplicationError {
    // PostgreSQL details can include private recovery and account values.
    ApplicationError::Port("password reset restoration operation failed".into())
}
