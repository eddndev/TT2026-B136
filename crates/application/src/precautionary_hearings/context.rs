use crate::{
    case_stages::{case_stage_digest, CaseStageEntry, CaseStageSnapshot},
    cases::{case_administration_digest, CaseAdministrationSnapshot},
    ApplicationError,
};
use domain::{
    case_administration::CaseAdministrativeStatus,
    case_stages::{CaseStage, CaseStageChange, StageTransition},
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{ArchiveEntry, DocumentHasher, Sha256Digest},
};

/// Complete observed context plus the historical administration referenced by its stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryContextMaterial {
    pub case_id: CaseId,
    pub administration: CaseAdministrationSnapshot,
    pub stage: CaseStageEntry,
    pub stage_administration: CaseAdministrationSnapshot,
}

/// Consistent historical material; this proves neither current access nor source admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryContext {
    material: PrecautionaryContextMaterial,
}

impl PrecautionaryContext {
    pub fn new(
        hasher: &dyn DocumentHasher,
        material: PrecautionaryContextMaterial,
    ) -> Result<Self, ApplicationError> {
        let observed = &material.administration;
        let origin = &material.stage_administration;
        for administration in [observed, origin] {
            if administration.case_id != material.case_id
                || administration.values.profile().is_none()
                || case_administration_digest(hasher, &administration.values)
                    != administration.values_digest
            {
                return Err(invalid("administration scope, profile or digest differs"));
            }
            provenance(administration.changed_at, &administration.changed_by.email)?;
        }
        if origin.values.status() != CaseAdministrativeStatus::Active
            || observed.revision < origin.revision
            || observed.changed_at < origin.changed_at
            || (observed.revision == origin.revision && observed != origin)
        {
            return Err(invalid("administration contradicts its stage origin"));
        }
        let stage = &material.stage;
        if stage.case_id() != material.case_id || stage.recorded_at() < origin.changed_at {
            return Err(invalid("stage scope or capture time differs"));
        }
        provenance(stage.recorded_at(), &stage.recorded_by().email)?;
        match stage {
            CaseStageEntry::Initial(initial) => {
                if initial.stage_revision.get() != 1
                    || initial.administration_revision.get() != 1
                    || origin.revision.get() != 1
                    || initial.administration_digest != origin.values_digest
                    || initial.recorded_at != origin.changed_at
                    || initial.recorded_by != origin.changed_by
                {
                    return Err(invalid("initial stage differs from its registration"));
                }
            }
            CaseStageEntry::Changed(changed) => {
                if changed.administration_revision != origin.revision
                    || changed.administration_digest != origin.values_digest
                    || case_stage_digest(hasher, &changed.values) != changed.values_digest
                {
                    return Err(invalid("changed stage differs from exact source values"));
                }
                validate_changed(changed)?;
            }
        }
        Ok(Self { material })
    }

    pub fn material(&self) -> &PrecautionaryContextMaterial {
        &self.material
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        super::context_encoding::encode(&self.material)
    }

    pub fn digest(&self, hasher: &dyn DocumentHasher) -> Sha256Digest {
        hasher.hash_bytes(&self.canonical_bytes())
    }
}

fn validate_changed(stage: &CaseStageSnapshot) -> Result<(), ApplicationError> {
    let revision = stage.stage_revision.get();
    let consistent = match &stage.values {
        CaseStageChange::Adopt(_) => revision == 1 && stage.from_stage.is_none(),
        CaseStageChange::Transition(StageTransition::ToIntermediate(_)) => {
            revision == 2 && stage.from_stage == Some(CaseStage::Investigation)
        }
        CaseStageChange::Transition(StageTransition::ToTrial(_)) => {
            matches!(revision, 2 | 3) && stage.from_stage == Some(CaseStage::Intermediate)
        }
    };
    if !consistent {
        return Err(invalid("stage predecessor or revision is impossible"));
    }
    stage.values.validate_recording_at(stage.recorded_at)?;
    let selected = stage.values.supports();
    if selected.len() != stage.supports.len() {
        return Err(invalid("stage support count differs"));
    }
    for (reference, captured) in selected.iter().zip(&stage.supports) {
        if reference.reference() != captured.reference || reference.digest() != captured.digest {
            return Err(invalid("exact stage support order or content differs"));
        }
        ArchiveEntry::new(captured.name.clone(), Vec::new())?;
    }
    Ok(())
}

fn provenance(at: OffsetDateTime, email: &str) -> Result<(), ApplicationError> {
    if at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
        return Err(invalid("capture time must be supported UTC"));
    }
    super::encoding::validate_actor_email(email)
}

fn invalid(message: &str) -> ApplicationError {
    ApplicationError::InvalidInput(format!("inconsistent precautionary context: {message}"))
}
