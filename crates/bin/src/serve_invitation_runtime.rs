//! One invitation email attempt per pause, supervised with all other consumers.
use crate::serve_deadline_runtime::RuntimeControl;
use application::{identity::invitations::InvitationDeliveryRun, ApplicationError};
use std::{sync::Arc, time::Duration};
pub(crate) type InvitationConsumer =
    Arc<dyn Fn() -> Result<InvitationDeliveryRun, ApplicationError> + Send + Sync>;
pub(crate) fn run(
    next: &dyn Fn() -> Result<InvitationDeliveryRun, ApplicationError>,
    control: &dyn RuntimeControl,
) -> Result<(), ApplicationError> {
    while !control.is_stopped() {
        match next() {
            Ok(_) => (),
            Err(ApplicationError::Port(_) | ApplicationError::ClassifiedPort { .. }) => {
                tracing::warn!("invitation delivery will retry after its pause");
            }
            Err(error) => return Err(error),
        }
        if !control.is_stopped() {
            control.wait(Duration::from_secs(1));
        }
    }
    Ok(())
}
