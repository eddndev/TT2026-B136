//! Strict reconstruction of historical hearing values from canonical projections.

use application::{hearings::*, ApplicationError};
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::participants::{ParticipantId, ParticipantRevision};
use serde_json::Value;
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

type Result<T> = std::result::Result<T, ApplicationError>;

/// Rejects altered fields and normalization that would hide a damaged projection.
pub fn values(canonical: &[u8], projection: &Value) -> Result<HearingValues> {
    if !(27..=10726).contains(&canonical.len()) || !canonical.starts_with(b"HEAR1") {
        return Err(inconsistent());
    }
    fields(
        projection,
        &[
            "kind",
            "time",
            "modality",
            "venue",
            "note",
            "participants",
            "conviction_basis",
        ],
    )?;
    let time = &projection["time"];
    fields(time, &["seconds", "offset_seconds"])?;
    let seconds = time["seconds"].as_i64().ok_or_else(inconsistent)?;
    let offset = time["offset_seconds"]
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(inconsistent)?;
    let at = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| inconsistent())?
        .checked_to_offset(UtcOffset::from_whole_seconds(offset).map_err(|_| inconsistent())?)
        .ok_or_else(inconsistent)?;
    let raw_venue = string(&projection["venue"])?;
    if raw_venue.len() > 2000 {
        return Err(inconsistent());
    }
    let venue = HearingVenue::new(raw_venue).map_err(|_| inconsistent())?;
    if venue.as_str() != raw_venue {
        return Err(inconsistent());
    }
    let refs = projection["participants"]
        .as_array()
        .ok_or_else(inconsistent)?;
    if refs.len() > 32 {
        return Err(inconsistent());
    }
    let participants = refs.iter().map(participant).collect::<Result<Vec<_>>>()?;
    if participants
        .windows(2)
        .any(|pair| pair[0].id().as_uuid() >= pair[1].id().as_uuid())
    {
        return Err(inconsistent());
    }
    let value = HearingValues::new(HearingValuesInput {
        kind: string(&projection["kind"])?
            .parse()
            .map_err(|_| inconsistent())?,
        scheduled_at: HearingTime::new(at).map_err(|_| inconsistent())?,
        modality: string(&projection["modality"])?
            .parse()
            .map_err(|_| inconsistent())?,
        venue,
        note: optional(&projection["note"], note)?,
        participants,
        conviction_basis: optional(&projection["conviction_basis"], basis)?,
    })
    .map_err(|_| inconsistent())?;
    if value.canonical_bytes() != canonical {
        return Err(inconsistent());
    }
    Ok(value)
}

fn participant(value: &Value) -> Result<HearingParticipantRef> {
    fields(value, &["id", "revision"])?;
    Ok(HearingParticipantRef::new(
        ParticipantId::from_uuid(uuid(&value["id"])?),
        ParticipantRevision::new(integer(&value["revision"])?).map_err(|_| inconsistent())?,
    ))
}
fn basis(value: &Value) -> Result<HearingConvictionBasis> {
    fields(value, &["statement", "document_id", "version", "digest"])?;
    Ok(HearingConvictionBasis::new(
        note(&value["statement"])?,
        HearingSupportRef::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(uuid(&value["document_id"])?),
                version: DocumentVersion::new(integer(&value["version"])?)
                    .map_err(|_| inconsistent())?,
            },
            digest(&value["digest"])?,
        ),
    ))
}
fn note(value: &Value) -> Result<HearingNote> {
    let raw = string(value)?;
    if raw.len() > 4000 {
        return Err(inconsistent());
    }
    let value = HearingNote::new(raw).map_err(|_| inconsistent())?;
    if value.as_str() != raw {
        return Err(inconsistent());
    }
    Ok(value)
}
fn optional<T>(value: &Value, parse: impl FnOnce(&Value) -> Result<T>) -> Result<Option<T>> {
    if value.is_null() {
        Ok(None)
    } else {
        parse(value).map(Some)
    }
}
fn fields(value: &Value, expected: &[&str]) -> Result<()> {
    let object = value.as_object().ok_or_else(inconsistent)?;
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err(inconsistent());
    }
    Ok(())
}
fn string(value: &Value) -> Result<&str> {
    value.as_str().ok_or_else(inconsistent)
}
fn integer(value: &Value) -> Result<u32> {
    value
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(inconsistent)
}
fn uuid(value: &Value) -> Result<Uuid> {
    let raw = string(value)?;
    if raw.len() != 36 {
        return Err(inconsistent());
    }
    let id = Uuid::parse_str(raw).map_err(|_| inconsistent())?;
    if id.to_string() != raw {
        return Err(inconsistent());
    }
    Ok(id)
}
fn digest(value: &Value) -> Result<Sha256Digest> {
    let raw = string(value)?;
    if raw.len() != 64 {
        return Err(inconsistent());
    }
    let digest = Sha256Digest::from_hex(raw).map_err(|_| inconsistent())?;
    if digest.to_hex() != raw {
        return Err(inconsistent());
    }
    Ok(digest)
}
fn inconsistent() -> ApplicationError {
    HearingError::StoredInconsistent("canonical values or projection are inconsistent".into())
        .into()
}
