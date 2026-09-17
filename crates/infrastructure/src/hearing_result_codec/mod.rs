//! Strict reconstruction of declared result values from canonical SQL projections.
mod helpers;
mod time;

use application::{hearing_results::*, ApplicationError};
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef};
use domain::participants::{ParticipantId, ParticipantRevision};
use helpers::*;
use serde_json::Value;

type Result<T> = std::result::Result<T, ApplicationError>;

/// Rejects projection damage, altered order, and normalization of stored fields.
pub fn values(canonical: &[u8], projection: &Value) -> Result<HearingResultValues> {
    if !(26..=146933).contains(&canonical.len()) || !canonical.starts_with(b"HRES1") {
        return Err(inconsistent());
    }
    fields(
        projection,
        &[
            "occurrence",
            "extent",
            "event_time",
            "summary",
            "attendees",
            "agreements",
            "provenance",
        ],
    )?;
    let attendees = projection["attendees"]
        .as_array()
        .ok_or_else(inconsistent)?;
    let agreements = projection["agreements"]
        .as_array()
        .ok_or_else(inconsistent)?;
    if attendees.len() > 32 || agreements.len() > 16 {
        return Err(inconsistent());
    }
    let attendees = attendees.iter().map(attendee).collect::<Result<Vec<_>>>()?;
    if attendees
        .windows(2)
        .any(|pair| pair[0].participant_id().as_uuid() >= pair[1].participant_id().as_uuid())
    {
        return Err(inconsistent());
    }
    let result = HearingResultValues::new(HearingResultValuesInput {
        occurrence: string(&projection["occurrence"])?
            .parse()
            .map_err(|_| inconsistent())?,
        extent: string(&projection["extent"])?
            .parse()
            .map_err(|_| inconsistent())?,
        event_time: time::declared(&projection["event_time"])?,
        summary: text(&projection["summary"])?,
        attendees,
        agreements: agreements
            .iter()
            .map(agreement)
            .collect::<Result<Vec<_>>>()?,
        provenance: provenance(&projection["provenance"])?,
    })
    .map_err(|_| inconsistent())?;
    if result.canonical_bytes() != canonical {
        return Err(inconsistent());
    }
    Ok(result)
}
fn attendee(value: &Value) -> Result<HearingResultAttendee> {
    fields(
        value,
        &["participant_id", "revision", "capacity", "observation"],
    )?;
    Ok(HearingResultAttendee::new(
        ParticipantId::from_uuid(uuid(&value["participant_id"])?),
        ParticipantRevision::new(counter(&value["revision"])?).map_err(|_| inconsistent())?,
        capacity(&value["capacity"])?,
        optional(&value["observation"], observation)?,
    ))
}
fn agreement(value: &Value) -> Result<HearingResultAgreement> {
    fields(value, &["id", "text"])?;
    Ok(HearingResultAgreement::new(
        HearingResultAgreementId::from_uuid(uuid(&value["id"])?),
        text(&value["text"])?,
    ))
}
fn provenance(value: &Value) -> Result<HearingResultProvenance> {
    fields(value, &["kind", "reference", "support"])?;
    HearingResultProvenance::new(
        string(&value["kind"])?
            .parse()
            .map_err(|_| inconsistent())?,
        optional(&value["reference"], reference)?,
        optional(&value["support"], support)?,
    )
    .map_err(|_| inconsistent())
}
fn support(value: &Value) -> Result<HearingResultSupportRef> {
    fields(value, &["document_id", "version", "digest"])?;
    Ok(HearingResultSupportRef::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(uuid(&value["document_id"])?),
            version: DocumentVersion::new(counter(&value["version"])?)
                .map_err(|_| inconsistent())?,
        },
        digest(&value["digest"])?,
    ))
}
fn inconsistent() -> ApplicationError {
    HearingResultError::StoredInconsistent("canonical values or projection are inconsistent".into())
        .into()
}
