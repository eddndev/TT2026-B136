use super::{declarations::person, helpers::*, inconsistent, Result};
use application::procedural_facts::*;
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    hearing_results::{HearingResultAgreementId, HearingResultId, HearingResultRevision},
    hearings::HearingId,
};
use serde_json::Value;

pub(super) fn hearing_reference(value: &Value) -> Result<FactHearingRef> {
    fields(
        value,
        &["hearing_id", "result_id", "revision", "agreement_id"],
    )?;
    Ok(FactHearingRef {
        hearing_id: HearingId::from_uuid(uuid(&value["hearing_id"])?),
        result_id: HearingResultId::from_uuid(uuid(&value["result_id"])?),
        revision: HearingResultRevision::new(counter(&value["revision"])?)
            .map_err(|_| inconsistent())?,
        agreement_id: optional(&value["agreement_id"], |v| {
            Ok(HearingResultAgreementId::from_uuid(uuid(v)?))
        })?,
    })
}
fn evidence(value: &Value) -> Result<FactEvidence> {
    fields(value, &["document_id", "version", "digest", "locator"])?;
    Ok(FactEvidence::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(uuid(&value["document_id"])?),
            version: DocumentVersion::new(counter(&value["version"])?)
                .map_err(|_| inconsistent())?,
        },
        digest(&value["digest"])?,
        label(&value["locator"])?,
    ))
}
pub(super) fn provenance(value: &Value) -> Result<FactProvenance> {
    match string(&value["kind"])? {
        "operator_note" => {
            fields(value, &["kind", "note"])?;
            Ok(FactProvenance::OperatorNote {
                note: text(&value["note"])?,
            })
        }
        "external_reference" => {
            fields(value, &["kind", "reference", "support"])?;
            Ok(FactProvenance::ExternalReference {
                reference: text(&value["reference"])?,
                support: optional(&value["support"], evidence)?,
            })
        }
        "hearing_result" => {
            fields(value, &["kind", "reference", "locator", "support"])?;
            Ok(FactProvenance::HearingResult {
                reference: hearing_reference(&value["reference"])?,
                locator: label(&value["locator"])?,
                support: optional(&value["support"], evidence)?,
            })
        }
        _ => Err(inconsistent()),
    }
}
pub(super) fn representation(value: &Value) -> Result<FactRepresentation> {
    match string(&value["kind"])? {
        "not_recorded" => {
            fields(value, &["kind", "reason"])?;
            Ok(FactRepresentation::NotRecorded(text(&value["reason"])?))
        }
        "declared" => {
            fields(
                value,
                &[
                    "kind",
                    "represented",
                    "representative",
                    "scope",
                    "provenance",
                ],
            )?;
            Ok(FactRepresentation::Declared {
                represented: person(&value["represented"])?,
                representative: person(&value["representative"])?,
                scope: text(&value["scope"])?,
                provenance: Box::new(provenance(&value["provenance"])?),
            })
        }
        _ => Err(inconsistent()),
    }
}
