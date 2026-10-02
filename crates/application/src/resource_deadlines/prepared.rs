use super::inconsistent;
use super::*;
use crate::{deadlines::*, identity::Principal, resource_activities::*, ApplicationError};
use domain::{clock::OffsetDateTime, crypto::DocumentHasher};
use std::sync::Arc;
pub struct PreparedResourceDeadline {
    pub(super) draft: ResourceDeadlineDraft,
    pub(super) material: ResourceDeadlineMaterial,
    pub(super) deadline: PreparedDeadlineChange,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub(super) actor: Principal,
}
impl PreparedResourceDeadline {
    pub fn command(&self) -> &ResourceDeadlineCommand {
        &self.draft.command
    }
    pub fn draft(&self) -> &ResourceDeadlineDraft {
        &self.draft
    }
    pub fn material(&self) -> &ResourceDeadlineMaterial {
        &self.material
    }
    pub fn into_result(
        self,
        at: OffsetDateTime,
    ) -> Result<ResourceDeadlineResult, ApplicationError> {
        let prepared = self.deadline;
        let deadline = DeadlineDetail {
            id: prepared.command().deadline_id,
            case_id: prepared.case_id(),
            revision: DeadlineRevision::initial(),
            definition: prepared.definition().clone(),
            calculation: prepared.calculation().clone(),
            tracking: prepared.tracking().cloned(),
            responsible: prepared.responsible().clone(),
            attention: prepared.attention().clone(),
            status: prepared.status(),
            reason: None,
            receipt: prepared.receipt(),
            recorded_at: at,
            recorded_by: self.draft.deadline.author.clone(),
        };
        deadline_receipt_matches(self.hasher.as_ref(), &deadline)?;
        let material = ResourceActivityMaterial {
            case_id: self.material.case_id,
            base: None,
            administration: self.material.administration,
            resource_head: self.material.resource_head,
            sources: ResourceActivitySources {
                resource: self.material.resource,
                act: self.material.act,
                target: ResourceActivityTargetDetail::Deadline(Box::new(deadline.clone())),
            },
        };
        let association = crate::resource_activities::prepare_activity_change(
            self.hasher.clone(),
            &self.actor,
            deadline.case_id,
            self.draft.command.resource.id,
            self.draft.association.command.clone(),
            material,
        )?
        .into_detail(at)?;
        let result = ResourceDeadlineResult {
            deadline,
            association,
            submission_digest: self.draft.submission_digest,
        };
        let actual = resource_deadline_result_draft(
            self.hasher.as_ref(),
            &self.actor,
            result.deadline.case_id,
            result.association.resource_id,
            &self.draft.command,
            &result,
        )?;
        if actual != self.draft {
            return Err(inconsistent(
                "committed contextual capture differs from review",
            ));
        }
        Ok(result)
    }
}
