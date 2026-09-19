use super::*;
use crate::ApplicationError;
use domain::identity::UserId;

/// Authorize active staff, recipient and current case access, then audit in one
/// transaction. Filter before pagination. No operation here records attention.
pub trait AlertStore: Send + Sync {
    fn preferences(&self, actor: UserId) -> Result<AlertPreferences, ApplicationError>;
    /// Replay the exact committed result for the same operation and content;
    /// reject reused operations with different content or stale new revisions.
    fn save_preferences(
        &self,
        actor: UserId,
        command: AlertPreferenceCommand,
    ) -> Result<AlertPreferences, ApplicationError>;
    fn list(&self, actor: UserId, query: AlertQuery) -> Result<AlertPage, ApplicationError>;
    fn get(&self, actor: UserId, id: AlertId) -> Result<AlertDetail, ApplicationError>;
    /// Idempotent by operation and alert. Preserve the original read timestamp.
    fn mark_read(
        &self,
        actor: UserId,
        command: AlertReadCommand,
    ) -> Result<AlertReadReceipt, ApplicationError>;
}
pub trait AlertWorkflow: Send + Sync {
    fn preferences(&self, token: &str) -> Result<AlertPreferences, ApplicationError>;
    fn save_preferences(
        &self,
        token: &str,
        command: AlertPreferenceCommand,
    ) -> Result<AlertPreferences, ApplicationError>;
    fn list(&self, token: &str, query: AlertQuery) -> Result<AlertPage, ApplicationError>;
    fn get(&self, token: &str, id: AlertId) -> Result<AlertDetail, ApplicationError>;
    fn mark_read(
        &self,
        token: &str,
        command: AlertReadCommand,
    ) -> Result<AlertReadReceipt, ApplicationError>;
}
/// Reconcile or activate bounded durable work using verified current evidence
/// and current recipients. Clock, cursor, notifications and audit share a tx.
pub trait AlertSchedulerStore: Send + Sync {
    fn run_next(&self) -> Result<AlertSchedulerRun, ApplicationError>;
}
/// A claim authenticates the current recipient, preferences and subject before
/// releasing the lock. Provider I/O happens outside this persistence boundary.
pub trait AlertDeliveryStore: Send + Sync {
    fn claim_next(&self) -> Result<Option<AlertDeliveryClaim>, ApplicationError>;
    /// Fence stale claims. Preserve uncertain outcomes for reconciliation;
    /// never change the idempotency key or frozen provider payload on retry.
    fn complete_attempt(&self, completion: AlertDeliveryCompletion)
        -> Result<(), ApplicationError>;
}
pub trait AlertEmailSender: Send + Sync {
    fn send(&self, message: &AlertEmailMessage) -> AlertEmailOutcome;
}
