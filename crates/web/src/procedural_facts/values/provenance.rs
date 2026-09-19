use super::super::{
    object::Object,
    request::{parse_digest, parse_uuid},
};
use super::{checked, label_text, text};
use crate::error::ApiError;
use application::procedural_facts::*;
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    hearing_results::{HearingResultAgreementId, HearingResultId, HearingResultRevision},
    hearings::HearingId,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Provenance {
    OperatorNote {
        note: String,
    },
    ExternalReference {
        reference: String,
        support: Option<Object<Evidence>>,
    },
    HearingResult {
        reference: Object<HearingRef>,
        locator: String,
        support: Option<Object<Evidence>>,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Evidence {
    document_id: String,
    version: u32,
    digest: String,
    locator: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HearingRef {
    hearing_id: String,
    result_id: String,
    revision: u32,
    agreement_id: Option<String>,
}
impl Provenance {
    pub(super) fn validate(self) -> Result<FactProvenance, ApiError> {
        Ok(match self {
            Self::OperatorNote { note } => FactProvenance::OperatorNote { note: text(&note)? },
            Self::ExternalReference { reference, support } => FactProvenance::ExternalReference {
                reference: text(&reference)?,
                support: support.map(|v| v.0.validate()).transpose()?,
            },
            Self::HearingResult {
                reference,
                locator,
                support,
            } => FactProvenance::HearingResult {
                reference: reference.0.validate()?,
                locator: label_text(&locator)?,
                support: support.map(|v| v.0.validate()).transpose()?,
            },
        })
    }
}
impl Evidence {
    pub(crate) fn validate(self) -> Result<FactEvidence, ApiError> {
        Ok(FactEvidence::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(parse_uuid(&self.document_id, "invalid_document_id")?),
                version: DocumentVersion::new(self.version)
                    .map_err(|_| ApiError::invalid_document_version())?,
            },
            parse_digest(&self.digest)?,
            label_text(&self.locator)?,
        ))
    }
}
impl HearingRef {
    fn validate(self) -> Result<FactHearingRef, ApiError> {
        Ok(FactHearingRef {
            hearing_id: HearingId::from_uuid(parse_uuid(&self.hearing_id, "invalid_hearing_id")?),
            result_id: HearingResultId::from_uuid(parse_uuid(
                &self.result_id,
                "invalid_hearing_result_id",
            )?),
            revision: checked(HearingResultRevision::new(self.revision))?,
            agreement_id: self
                .agreement_id
                .map(|v| {
                    parse_uuid(&v, "invalid_hearing_result_agreement_id")
                        .map(HearingResultAgreementId::from_uuid)
                })
                .transpose()?,
        })
    }
}
pub(crate) fn evidence(value: &FactEvidence) -> Value {
    json!({"document_id":value.reference().id.to_string(),"version":value.reference().version.get(),
        "digest":value.digest().to_hex(),"locator":value.locator().as_str()})
}
pub(super) fn project(value: &FactProvenance) -> Value {
    match value {
        FactProvenance::OperatorNote { note } => {
            json!({"kind":"operator_note","note":note.as_str()})
        }
        FactProvenance::ExternalReference { reference, support } => {
            json!({"kind":"external_reference",
            "reference":reference.as_str(),"support":support.as_ref().map(evidence)})
        }
        FactProvenance::HearingResult {
            reference,
            locator,
            support,
        } => json!({"kind":"hearing_result",
            "reference":{"hearing_id":reference.hearing_id.to_string(),"result_id":reference.result_id.to_string(),
                "revision":reference.revision.get(),"agreement_id":reference.agreement_id.map(|v|v.to_string())},
            "locator":locator.as_str(),"support":support.as_ref().map(evidence)}),
    }
}
