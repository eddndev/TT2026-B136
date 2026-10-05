use super::primitives::{digest, uuid};
use crate::error::ApiError;
use domain::{
    precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef},
    typed_participants::{CaseSubjectId, SubjectRevision, SubjectRevisionRef},
};
use serde::Deserialize;

pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body("invalid_measure_request", "invalid measure request")
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reference {
    id: String,
    revision: u32,
    capture_digest: String,
}
impl Reference {
    pub(super) fn validate(self) -> Result<PrecautionaryMeasureRef, ApiError> {
        Ok(PrecautionaryMeasureRef::new(
            MeasureId::from_uuid(uuid(&self.id)?),
            MeasureRevision::new(self.revision).map_err(|_| invalid())?,
            digest(&self.capture_digest)?,
        ))
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Subject {
    id: String,
    revision: u32,
    values_digest: String,
}
impl Subject {
    pub(super) fn validate(self) -> Result<SubjectRevisionRef, ApiError> {
        Ok(SubjectRevisionRef {
            id: CaseSubjectId::from_uuid(uuid(&self.id)?),
            revision: SubjectRevision::new(self.revision).map_err(|_| invalid())?,
            values_digest: digest(&self.values_digest)?,
        })
    }
}
