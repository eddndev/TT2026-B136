use super::*;
use crate::{
    case_stages::StageSupportSnapshot, cases::CaseActorSnapshot, identity::Principal,
    ApplicationError,
};
use domain::{crypto::Sha256Digest, procedural_facts::FactSupportRef};

impl ProceduralResourceService {
    pub(super) fn prepared(
        &self,
        actor: &Principal,
        command: ResourceCommand,
        mut material: ResourceMaterial,
    ) -> Result<PreparedResourceChange, ApplicationError> {
        let case_id = material.case_id;
        super::context::material(self.hasher.as_ref(), case_id, &command, &material)?;
        let base = material.base.clone();
        let values = match &command.change {
            ResourceChange::Register { values } | ResourceChange::Correct { values, .. } => {
                values.clone()
            }
            _ => base
                .as_ref()
                .ok_or_else(|| inconsistent("resource command lacks its base"))?
                .values
                .clone(),
        };
        let sources = if matches!(
            command.action(),
            ResourceAction::Register | ResourceAction::Correct
        ) {
            let mut resolved =
                super::sources::resolve(self.hasher.as_ref(), case_id, &values, &material)?;
            let retained = base
                .as_ref()
                .map(|b| b.sources.supports.as_slice())
                .unwrap_or(&[]);
            resolved.supports = self.admit(&mut material, &values.direct_supports(), retained)?;
            if let Some(base) = &base {
                super::sources::retained(&base.sources, &resolved)?;
            }
            resolved
        } else {
            base.as_ref()
                .ok_or_else(|| inconsistent("resource retained sources are absent"))?
                .sources
                .clone()
        };
        super::sources::validate_selection(case_id, &values, &sources)?;
        let act = self.prepare_act(&command, &mut material)?;
        if matches!(
            command.action(),
            ResourceAction::Archive | ResourceAction::Reactivate
        ) && !material.records.is_empty()
        {
            return Err(inconsistent(
                "organizational resource command contains new documents",
            ));
        }
        let status = if command.action() == ResourceAction::Archive {
            ResourceStatus::Archived
        } else {
            ResourceStatus::Active
        };
        let mut draft = ResourceDraft {
            case_id,
            result_revision: command.result_revision()?,
            command,
            values,
            status,
            sources,
            act,
            previous: base.as_ref().map(|b| ResourceRevisionRef {
                revision: b.revision,
                capture_digest: b.receipt.capture_digest,
            }),
            recorded_by: CaseActorSnapshot {
                id: actor.id,
                email: actor.email.clone(),
            },
            observed_administration: material.administration.clone(),
            observed_stage: material.stage.clone(),
            submission_digest: Sha256Digest::from_array([0; 32]),
        };
        draft.submission_digest = self
            .hasher
            .hash_bytes(&resource_submission_bytes(self.hasher.as_ref(), &draft)?);
        Ok(PreparedResourceChange {
            material,
            draft,
            hasher: self.hasher.clone(),
        })
    }
    fn admit(
        &self,
        material: &mut ResourceMaterial,
        selected: &[FactSupportRef],
        retained: &[StageSupportSnapshot],
    ) -> Result<Vec<StageSupportSnapshot>, ApplicationError> {
        let mut new = vec![];
        let mut supports = vec![];
        for reference in selected {
            if let Some(old) = retained
                .iter()
                .find(|s| s.reference == reference.reference())
            {
                if old.digest != reference.digest() {
                    return Err(ApplicationError::StageSupportDigestMismatch);
                }
                supports.push(old.clone());
            } else {
                new.push(*reference);
            }
        }
        super::sources::check_records(material, &new)?;
        if !material.records.is_empty() {
            let formats = self.processor.validate_support_batch(
                &material.records,
                &self.limits,
                self.validator.as_ref(),
            )?;
            supports.extend(super::sources::admitted(material, formats)?);
        }
        supports.sort_by_key(|s| (s.reference.id.as_uuid(), s.reference.version.get()));
        super::sources::validate_supports(selected, &supports)?;
        Ok(supports)
    }
    fn prepare_act(
        &self,
        command: &ResourceCommand,
        material: &mut ResourceMaterial,
    ) -> Result<Option<ResourceActCapture>, ApplicationError> {
        let (id, values, expected) = match &command.change {
            ResourceChange::RecordAct { act_id, values, .. } => (*act_id, values, None),
            ResourceChange::CorrectAct {
                act_id,
                expected_act_revision,
                values,
                ..
            } => (*act_id, values, Some(*expected_act_revision)),
            _ => return Ok(None),
        };
        let prior = material.act_base.clone();
        let (revision, previous, retained) = match (expected, prior.as_ref()) {
            (None, None) => (ResourceActRevision::initial(), None, vec![]),
            (Some(expected), Some(previous)) => {
                resource_receipt_matches(self.hasher.as_ref(), previous)?;
                let act = previous
                    .act
                    .as_ref()
                    .ok_or_else(|| inconsistent("resource act base has no act"))?;
                if previous.case_id != material.case_id
                    || previous.id != command.resource_id
                    || previous.revision.get() > command.expected_revision()
                    || act.id != id
                    || material
                        .base
                        .as_ref()
                        .is_some_and(|b| previous.recorded_at > b.recorded_at)
                {
                    return Err(inconsistent(
                        "resource act base scope or chronology differs",
                    ));
                }
                if act.revision != expected {
                    return Err(ProceduralResourceError::RevisionConflict.into());
                }
                let next = act.revision.next().ok_or_else(|| {
                    ApplicationError::InvalidInput("resource act revision exhausted".into())
                })?;
                (
                    next,
                    Some(ResourceRevisionRef {
                        revision: previous.revision,
                        capture_digest: previous.receipt.capture_digest,
                    }),
                    act.supports.clone(),
                )
            }
            _ => return Err(ProceduralResourceError::RevisionConflict.into()),
        };
        let supports = self.admit(material, &values.direct_supports(), &retained)?;
        super::sources::retained_supports(&retained, &supports)?;
        Ok(Some(ResourceActCapture {
            id,
            revision,
            values: values.clone(),
            supports,
            previous,
        }))
    }
}
