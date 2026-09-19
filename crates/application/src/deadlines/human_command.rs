use super::{DeadlineChange, DeadlineCommand, DeadlineError};
use crate::deadline_tracking::{TrackingPolicies, TrackingPolicy};
use domain::procedural_facts::FactDeclaration;

/// A human decision declares how each selected dependency will be tracked.
/// Authors, observed heads and technical causes come from verified server state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineHumanCommand {
    command: DeadlineCommand,
    policies: Option<TrackingPolicies>,
}

impl DeadlineHumanCommand {
    pub fn new(
        command: DeadlineCommand,
        policies: Option<TrackingPolicies>,
    ) -> Result<Self, DeadlineError> {
        match &command.change {
            DeadlineChange::Register { definition }
            | DeadlineChange::Correct { definition, .. } => {
                let policies = policies.ok_or(DeadlineError::Invalid("tracking"))?;
                let source_present =
                    matches!(definition.input.selection.source, FactDeclaration::Known(_));
                for (policy, present, field) in [
                    (policies.profile, true, "tracking.profile"),
                    (policies.source, source_present, "tracking.source"),
                    (
                        policies.calendar,
                        definition.input.calendar.is_some(),
                        "tracking.calendar",
                    ),
                ] {
                    let declared = policy != TrackingPolicy::Undetermined;
                    if declared != present {
                        return Err(DeadlineError::Invalid(field));
                    }
                }
            }
            DeadlineChange::SetAttention { .. } | DeadlineChange::Retire { .. } => {
                if policies.is_some() {
                    return Err(DeadlineError::Invalid("tracking"));
                }
            }
        }
        Ok(Self { command, policies })
    }

    pub fn into_parts(self) -> (DeadlineCommand, Option<TrackingPolicies>) {
        (self.command, self.policies)
    }
}
