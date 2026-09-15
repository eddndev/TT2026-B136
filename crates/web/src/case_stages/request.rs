use super::date::DeclaredTime;
use crate::error::ApiError;
use application::{case_stages::*, ApplicationError};
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    DomainError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Support {
    document_id: String,
    version: u32,
    digest: String,
}
impl Support {
    fn validate(self) -> Result<StageSupportRef, ApiError> {
        let id = Uuid::parse_str(&self.document_id)
            .map(DocumentId::from_uuid)
            .map_err(|_| ApiError::invalid_document_id())?;
        let version =
            DocumentVersion::new(self.version).map_err(|_| ApiError::invalid_document_version())?;
        let digest = Sha256Digest::from_hex(&self.digest).map_err(|_| {
            ApiError::invalid_body(
                "invalid_stage_support_digest",
                "stage support digest must be 64 hexadecimal characters",
            )
        })?;
        Ok(StageSupportRef::new(
            DocumentVersionRef { id, version },
            digest,
        ))
    }
}
impl From<StageSupportRef> for Support {
    fn from(value: StageSupportRef) -> Self {
        Self {
            document_id: value.reference().id.to_string(),
            version: value.reference().version.get(),
            digest: value.digest().to_hex(),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Adoption {
    expected_revision: u32,
    stage: String,
    known_at: DeclaredTime,
    reason: String,
    support: Support,
}
impl Adoption {
    pub fn validate(self) -> Result<(CaseStageExpectation, StageAdoption), ApiError> {
        if self.expected_revision != 0 {
            return Err(ApplicationError::Domain(DomainError::InvalidCaseStageRevision).into());
        }
        let stage = self.stage.parse().map_err(ApplicationError::from)?;
        let known_at = self.known_at.validate().map_err(ApplicationError::from)?;
        let reason = StageNote::new(&self.reason).map_err(ApplicationError::from)?;
        Ok((
            CaseStageExpectation::Unregistered,
            StageAdoption::new(stage, known_at, reason, self.support.validate()?),
        ))
    }
}
#[derive(Deserialize)]
#[serde(tag = "target", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Transition {
    Intermediate {
        expected_revision: u32,
        accusation_declared_at: DeclaredTime,
        accusation: Support,
        note: Option<String>,
    },
    Trial {
        expected_revision: u32,
        opening_order_issued_at: DeclaredTime,
        opening_order: Support,
        received_at: DeclaredTime,
        receiving_court: String,
        receipt_reference: Option<String>,
        receipt_support: Option<Support>,
        note: Option<String>,
    },
}
impl Transition {
    pub fn validate(self) -> Result<(CaseStageRevision, StageTransition), ApiError> {
        match self {
            Self::Intermediate {
                expected_revision,
                accusation_declared_at,
                accusation,
                note,
            } => Ok((
                CaseStageRevision::new(expected_revision).map_err(ApplicationError::from)?,
                StageTransition::to_intermediate(
                    accusation_declared_at
                        .validate()
                        .map_err(ApplicationError::from)?,
                    accusation.validate()?,
                    StageNote::optional(note.as_deref()).map_err(ApplicationError::from)?,
                ),
            )),
            Self::Trial {
                expected_revision,
                opening_order_issued_at,
                opening_order,
                received_at,
                receiving_court,
                receipt_reference,
                receipt_support,
                note,
            } => Ok((
                CaseStageRevision::new(expected_revision).map_err(ApplicationError::from)?,
                StageTransition::to_trial(
                    opening_order_issued_at
                        .validate()
                        .map_err(ApplicationError::from)?,
                    opening_order.validate()?,
                    received_at.validate().map_err(ApplicationError::from)?,
                    StageCourt::new(&receiving_court).map_err(ApplicationError::from)?,
                    StageReceiptReference::optional(receipt_reference.as_deref())
                        .map_err(ApplicationError::from)?,
                    receipt_support.map(Support::validate).transpose()?,
                    StageNote::optional(note.as_deref()).map_err(ApplicationError::from)?,
                )
                .map_err(ApplicationError::from)?,
            )),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HistoryQuery {
    limit: Option<u32>,
    before_revision: Option<u32>,
}
impl HistoryQuery {
    pub fn validate(self) -> Result<CaseStageQuery, ApiError> {
        CaseStageQuery::new(self.limit.unwrap_or(20), self.before_revision).map_err(|_| {
            ApiError::invalid_body(
                "invalid_query",
                "stage history requires limit 1..100 and a positive revision cursor",
            )
        })
    }
}
