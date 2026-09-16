use super::*;

/// Withdrawal has no replacement values; its committed content comes from its base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactChange<V> {
    Record {
        values: Box<V>,
    },
    Correct {
        expected_revision: FactRevision,
        values: Box<V>,
        reason: FactText,
    },
    Withdraw {
        expected_revision: FactRevision,
        reason: FactText,
    },
}
impl<V> FactChange<V> {
    pub fn record(values: V) -> Self {
        Self::Record {
            values: Box::new(values),
        }
    }
    pub fn correct(expected_revision: FactRevision, values: V, reason: FactText) -> Self {
        Self::Correct {
            expected_revision,
            values: Box::new(values),
            reason,
        }
    }
    pub fn withdraw(expected_revision: FactRevision, reason: FactText) -> Self {
        Self::Withdraw {
            expected_revision,
            reason,
        }
    }
    pub const fn action(&self) -> FactAction {
        match self {
            Self::Record { .. } => FactAction::Record,
            Self::Correct { .. } => FactAction::Correct,
            Self::Withdraw { .. } => FactAction::Withdraw,
        }
    }
    pub const fn expected_revision(&self) -> u32 {
        match self {
            Self::Record { .. } => 0,
            Self::Correct {
                expected_revision, ..
            }
            | Self::Withdraw {
                expected_revision, ..
            } => expected_revision.get(),
        }
    }
    pub fn result_revision(&self) -> Result<FactRevision, ProceduralFactError> {
        match self {
            Self::Record { .. } => Ok(FactRevision::initial()),
            Self::Correct {
                expected_revision, ..
            }
            | Self::Withdraw {
                expected_revision, ..
            } => expected_revision
                .next()
                .ok_or(ProceduralFactError::RevisionExhausted),
        }
    }
    pub const fn reason(&self) -> Option<&FactText> {
        match self {
            Self::Record { .. } => None,
            Self::Correct { reason, .. } | Self::Withdraw { reason, .. } => Some(reason),
        }
    }
    pub fn values(&self) -> Option<&V> {
        match self {
            Self::Record { values } | Self::Correct { values, .. } => Some(values.as_ref()),
            Self::Withdraw { .. } => None,
        }
    }
    /// Pure state check; the repository must repeat it against the locked head.
    pub fn validate_base(
        &self,
        base: Option<(FactRevision, FactStatus)>,
    ) -> Result<(), ProceduralFactError> {
        self.result_revision()?;
        match (self.action(), base) {
            (FactAction::Record, None) => Ok(()),
            (FactAction::Record, Some(_)) => Err(ProceduralFactError::RevisionConflict),
            (_, None) => Err(ProceduralFactError::NotFound),
            (_, Some((revision, _))) if revision.get() != self.expected_revision() => {
                Err(ProceduralFactError::RevisionConflict)
            }
            (_, Some((_, FactStatus::Withdrawn))) => Err(ProceduralFactError::AlreadyWithdrawn),
            (_, Some(_)) => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionCommand {
    operation_id: FactOperationId,
    resolution_id: ResolutionId,
    change: FactChange<ResolutionValues>,
}
impl ResolutionCommand {
    pub fn new(
        operation_id: FactOperationId,
        resolution_id: ResolutionId,
        change: FactChange<ResolutionValues>,
    ) -> Self {
        Self {
            operation_id,
            resolution_id,
            change,
        }
    }
    pub const fn operation_id(&self) -> FactOperationId {
        self.operation_id
    }
    pub const fn resolution_id(&self) -> ResolutionId {
        self.resolution_id
    }
    pub const fn change(&self) -> &FactChange<ResolutionValues> {
        &self.change
    }
    pub const fn action(&self) -> FactAction {
        self.change.action()
    }
    pub const fn expected_revision(&self) -> u32 {
        self.change.expected_revision()
    }
    pub fn result_revision(&self) -> Result<FactRevision, ProceduralFactError> {
        self.change.result_revision()
    }
    pub const fn reason(&self) -> Option<&FactText> {
        self.change.reason()
    }
    pub fn validate_base(
        &self,
        base: Option<(FactRevision, FactStatus)>,
    ) -> Result<(), ProceduralFactError> {
        self.change.validate_base(base)
    }
}

/// A correction may select another exact revision, never another resolution root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationCommand {
    operation_id: FactOperationId,
    notification_id: NotificationId,
    resolution_id: ResolutionId,
    change: FactChange<NotificationValues>,
}
impl NotificationCommand {
    pub fn new(
        operation_id: FactOperationId,
        notification_id: NotificationId,
        resolution_id: ResolutionId,
        change: FactChange<NotificationValues>,
    ) -> Result<Self, ProceduralFactError> {
        if change
            .values()
            .is_some_and(|v| v.resolution().id != resolution_id)
        {
            return Err(ProceduralFactError::InvalidReference);
        }
        Ok(Self {
            operation_id,
            notification_id,
            resolution_id,
            change,
        })
    }
    pub const fn operation_id(&self) -> FactOperationId {
        self.operation_id
    }
    pub const fn notification_id(&self) -> NotificationId {
        self.notification_id
    }
    pub const fn resolution_id(&self) -> ResolutionId {
        self.resolution_id
    }
    pub const fn change(&self) -> &FactChange<NotificationValues> {
        &self.change
    }
    pub const fn action(&self) -> FactAction {
        self.change.action()
    }
    pub const fn expected_revision(&self) -> u32 {
        self.change.expected_revision()
    }
    pub fn result_revision(&self) -> Result<FactRevision, ProceduralFactError> {
        self.change.result_revision()
    }
    pub const fn reason(&self) -> Option<&FactText> {
        self.change.reason()
    }
    pub fn validate_base(
        &self,
        base: Option<(FactRevision, FactStatus)>,
    ) -> Result<(), ProceduralFactError> {
        self.change.validate_base(base)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProceduralFactCommand {
    Resolution(ResolutionCommand),
    Notification(NotificationCommand),
}
impl ProceduralFactCommand {
    pub const fn target(&self) -> FactTarget {
        match self {
            Self::Resolution(v) => FactTarget::Resolution(v.resolution_id()),
            Self::Notification(v) => FactTarget::Notification {
                id: v.notification_id(),
                resolution_id: v.resolution_id(),
            },
        }
    }
    pub const fn operation_id(&self) -> FactOperationId {
        match self {
            Self::Resolution(v) => v.operation_id(),
            Self::Notification(v) => v.operation_id(),
        }
    }
    pub const fn action(&self) -> FactAction {
        match self {
            Self::Resolution(v) => v.action(),
            Self::Notification(v) => v.action(),
        }
    }
    pub const fn expected_revision(&self) -> u32 {
        match self {
            Self::Resolution(v) => v.expected_revision(),
            Self::Notification(v) => v.expected_revision(),
        }
    }
    pub fn result_revision(&self) -> Result<FactRevision, ProceduralFactError> {
        match self {
            Self::Resolution(v) => v.result_revision(),
            Self::Notification(v) => v.result_revision(),
        }
    }
    pub const fn reason(&self) -> Option<&FactText> {
        match self {
            Self::Resolution(v) => v.reason(),
            Self::Notification(v) => v.reason(),
        }
    }
    pub fn validate_base(
        &self,
        base: Option<(FactRevision, FactStatus)>,
    ) -> Result<(), ProceduralFactError> {
        match self {
            Self::Resolution(v) => v.validate_base(base),
            Self::Notification(v) => v.validate_base(base),
        }
    }
}
