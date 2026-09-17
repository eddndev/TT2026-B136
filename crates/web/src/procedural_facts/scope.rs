use super::request;
use crate::error::ApiError;
use application::procedural_facts::*;
use domain::cases::CaseId;
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct RoutePath {
    pub case: String,
    pub resolution: Option<String>,
    pub notification: Option<String>,
    pub revision: Option<String>,
}
#[derive(Clone, Copy)]
pub(super) struct Scope {
    pub case: CaseId,
    pub family: FactFamily,
    pub parent: Option<ResolutionId>,
    pub target: Option<FactTarget>,
}
impl Scope {
    pub fn parse(family: FactFamily, input: &RoutePath) -> Result<Self, ApiError> {
        let case = CaseId::from_uuid(request::parse_uuid(&input.case, "invalid_case_id")?);
        let resolution = input
            .resolution
            .as_deref()
            .map(|value| {
                request::parse_uuid(value, "invalid_resolution_id").map(ResolutionId::from_uuid)
            })
            .transpose()?;
        let (parent, target) = match family {
            FactFamily::Resolution => (None, resolution.map(FactTarget::Resolution)),
            FactFamily::Notification => {
                let resolution_id = resolution.ok_or_else(ApiError::internal)?;
                let target = input
                    .notification
                    .as_deref()
                    .map(|value| {
                        request::parse_uuid(value, "invalid_notification_id").map(|id| {
                            FactTarget::Notification {
                                id: NotificationId::from_uuid(id),
                                resolution_id,
                            }
                        })
                    })
                    .transpose()?;
                (Some(resolution_id), target)
            }
        };
        Ok(Self {
            case,
            family,
            parent,
            target,
        })
    }
    pub fn target(self) -> Result<FactTarget, ApiError> {
        self.target.ok_or_else(ApiError::internal)
    }
    pub fn accepts(self, command: &ProceduralFactCommand) -> bool {
        let target = command.target();
        if target.family() != self.family || self.target.is_some_and(|expected| target != expected)
        {
            return false;
        }
        match target {
            FactTarget::Resolution(_) => self.parent.is_none(),
            FactTarget::Notification { resolution_id, .. } => self.parent == Some(resolution_id),
        }
    }
}
pub(super) fn revision(value: &str) -> Result<FactRevision, ApiError> {
    if value.is_empty() || value.len() > 10 || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid_revision());
    }
    FactRevision::new(value.parse().map_err(|_| invalid_revision())?)
        .map_err(|_| invalid_revision())
}
fn invalid_revision() -> ApiError {
    ApiError::invalid_body(
        "invalid_procedural_fact_revision",
        "revision must be a positive u32",
    )
}
