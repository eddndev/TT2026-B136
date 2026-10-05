//! Strict reconstruction of declared precautionary hearing values from stored projections.

use application::{precautionary_hearings::PrecautionaryHearingError, ApplicationError};
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    hearings::{HearingNote, HearingParticipantRef, HearingSupportRef, HearingTime, HearingVenue},
    participants::{ParticipantId, ParticipantRevision},
    precautionary_hearings::{
        MeasureId, MeasureRevision, PrecautionaryHearingSchedulingBasis,
        PrecautionaryHearingValues, PrecautionaryHearingValuesInput, PrecautionaryMeasureRef,
    },
};
use serde_json::Value;
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

type Result<T> = std::result::Result<T, ApplicationError>;
const MAX_CANONICAL_BYTES: usize = 16_395;

/// Rejects inconsistent bytes, unknown fields and normalization of persisted values.
pub fn values(canonical: &[u8], projection: &Value) -> Result<PrecautionaryHearingValues> {
    if !(6..=MAX_CANONICAL_BYTES).contains(&canonical.len()) || !canonical.starts_with(b"PHEAR1") {
        return Err(inconsistent());
    }
    fields(
        projection,
        &[
            "purpose",
            "time",
            "modality",
            "venue",
            "note",
            "participants",
            "scheduling_basis",
            "review_targets",
        ],
    )?;
    let participant_values = array(&projection["participants"])?;
    let target_values = array(&projection["review_targets"])?;
    let participants = participant_values
        .iter()
        .map(participant)
        .collect::<Result<Vec<_>>>()?;
    if participants
        .windows(2)
        .any(|pair| pair[0].id().as_uuid() >= pair[1].id().as_uuid())
    {
        return Err(inconsistent());
    }
    let targets = target_values
        .iter()
        .map(target)
        .collect::<Result<Vec<_>>>()?;
    if targets
        .windows(2)
        .any(|pair| pair[0].id().as_uuid() >= pair[1].id().as_uuid())
    {
        return Err(inconsistent());
    }
    let raw_venue = string(&projection["venue"])?;
    if raw_venue.len() > 2000 {
        return Err(inconsistent());
    }
    let venue = HearingVenue::new(raw_venue).map_err(|_| inconsistent())?;
    if venue.as_str() != raw_venue {
        return Err(inconsistent());
    }
    let value = PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
        purpose: string(&projection["purpose"])?
            .parse()
            .map_err(|_| inconsistent())?,
        scheduled_at: scheduled_at(&projection["time"])?,
        modality: string(&projection["modality"])?
            .parse()
            .map_err(|_| inconsistent())?,
        venue,
        note: if projection["note"].is_null() {
            None
        } else {
            Some(note(&projection["note"])?)
        },
        participants,
        scheduling_basis: basis(&projection["scheduling_basis"])?,
        review_targets: targets,
    })
    .map_err(|_| inconsistent())?;
    if value.canonical_bytes() != canonical {
        return Err(inconsistent());
    }
    Ok(value)
}

fn scheduled_at(value: &Value) -> Result<HearingTime> {
    fields(value, &["seconds", "offset_seconds"])?;
    let seconds = value["seconds"].as_i64().ok_or_else(inconsistent)?;
    let offset = value["offset_seconds"]
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(inconsistent)?;
    let at = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| inconsistent())?
        .checked_to_offset(UtcOffset::from_whole_seconds(offset).map_err(|_| inconsistent())?)
        .ok_or_else(inconsistent)?;
    HearingTime::new(at).map_err(|_| inconsistent())
}

fn participant(value: &Value) -> Result<HearingParticipantRef> {
    fields(value, &["id", "revision"])?;
    Ok(HearingParticipantRef::new(
        ParticipantId::from_uuid(uuid(&value["id"])?),
        ParticipantRevision::new(integer(&value["revision"])?).map_err(|_| inconsistent())?,
    ))
}

fn target(value: &Value) -> Result<PrecautionaryMeasureRef> {
    fields(value, &["id", "revision", "digest"])?;
    Ok(PrecautionaryMeasureRef::new(
        MeasureId::from_uuid(uuid(&value["id"])?),
        MeasureRevision::new(integer(&value["revision"])?).map_err(|_| inconsistent())?,
        digest(&value["digest"])?,
    ))
}

fn basis(value: &Value) -> Result<PrecautionaryHearingSchedulingBasis> {
    fields(
        value,
        &["statement", "document_id", "version", "digest", "locator"],
    )?;
    Ok(PrecautionaryHearingSchedulingBasis::new(
        note(&value["statement"])?,
        HearingSupportRef::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(uuid(&value["document_id"])?),
                version: DocumentVersion::new(integer(&value["version"])?)
                    .map_err(|_| inconsistent())?,
            },
            digest(&value["digest"])?,
        ),
        note(&value["locator"])?,
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

fn fields(value: &Value, expected: &[&str]) -> Result<()> {
    let object = value.as_object().ok_or_else(inconsistent)?;
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err(inconsistent());
    }
    Ok(())
}

fn array(value: &Value) -> Result<&[Value]> {
    let values = value.as_array().ok_or_else(inconsistent)?;
    if values.len() > 32 {
        return Err(inconsistent());
    }
    Ok(values)
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
    PrecautionaryHearingError::StoredInconsistent(
        "canonical values or projection are inconsistent".into(),
    )
    .into()
}

/// Bounded values projection stored alongside the independent PHEAR1 commitment.
pub fn view(value: &PrecautionaryHearingValues) -> Value {
    let basis = value.scheduling_basis();
    let support = basis.support();
    serde_json::json!({
        "purpose":value.purpose().as_str(),
        "time":{"seconds":value.scheduled_at().value().unix_timestamp(),
            "offset_seconds":value.scheduled_at().value().offset().whole_seconds()},
        "modality":value.modality().as_str(),
        "venue":value.venue().as_str(),
        "note":value.note().map(HearingNote::as_str),
        "participants":value.participants().iter().map(|p|serde_json::json!({
            "id":p.id().to_string(),"revision":p.revision().get()})).collect::<Vec<_>>(),
        "scheduling_basis":{"statement":basis.statement().as_str(),
            "document_id":support.reference().id.to_string(),
            "version":support.reference().version.get(),"digest":support.digest().to_hex(),
            "locator":basis.locator().as_str()},
        "review_targets":value.review_targets().iter().map(|r|serde_json::json!({
            "id":r.id().to_string(),"revision":r.revision().get(),
            "digest":r.digest().to_hex()})).collect::<Vec<_>>()
    })
}
