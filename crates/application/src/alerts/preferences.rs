use super::AlertOperationId;
use crate::ApplicationError;
use domain::{alerts::AlertAnticipations, clock::OffsetDateTime, identity::UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertChannels {
    pub internal: bool,
    pub email: bool,
}
impl Default for AlertChannels {
    fn default() -> Self {
        Self {
            internal: true,
            email: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlertFamilyPreferences {
    pub anticipations: AlertAnticipations,
    pub channels: AlertChannels,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlertPreferenceValues {
    pub hearing_upcoming: AlertFamilyPreferences,
    pub deadline_upcoming: AlertFamilyPreferences,
    pub overdue_unattended: AlertChannels,
    pub review_required: AlertChannels,
    pub due_changed_soon: AlertChannels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertEmailTransport {
    Ready,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertPreferenceCommand {
    pub operation_id: AlertOperationId,
    pub expected_revision: u32,
    pub values: AlertPreferenceValues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertPreferenceReceipt {
    pub operation_id: AlertOperationId,
    pub expected_revision: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertPreferences {
    pub user_id: UserId,
    /// Zero denotes defaults with no persisted user mutation.
    pub revision: u32,
    pub values: AlertPreferenceValues,
    pub updated_at: Option<OffsetDateTime>,
    pub receipt: Option<AlertPreferenceReceipt>,
    pub email_transport: AlertEmailTransport,
}
impl AlertPreferences {
    pub fn initial(user_id: UserId, email_transport: AlertEmailTransport) -> Self {
        Self {
            user_id,
            revision: 0,
            values: AlertPreferenceValues::default(),
            updated_at: None,
            receipt: None,
            email_transport,
        }
    }
    pub fn validate(&self, actor: UserId) -> Result<(), ApplicationError> {
        if self.user_id != actor {
            return Err(super::validation::stored("preference owner differs"));
        }
        match (self.revision, self.updated_at, self.receipt) {
            (0, None, None) if self.values == AlertPreferenceValues::default() => Ok(()),
            (revision, Some(at), Some(receipt))
                if revision > 0
                    && receipt.expected_revision.checked_add(1) == Some(revision)
                    && super::validation::utc_time(at) =>
            {
                Ok(())
            }
            _ => Err(super::validation::stored(
                "preference revision evidence differs",
            )),
        }
    }
    pub fn validate_command(
        &self,
        actor: UserId,
        command: &AlertPreferenceCommand,
    ) -> Result<(), ApplicationError> {
        self.validate(actor)?;
        if self.values != command.values
            || self.revision == 0
            || self.receipt
                != Some(AlertPreferenceReceipt {
                    operation_id: command.operation_id,
                    expected_revision: command.expected_revision,
                })
        {
            return Err(super::validation::stored(
                "preference command receipt differs",
            ));
        }
        Ok(())
    }
}
