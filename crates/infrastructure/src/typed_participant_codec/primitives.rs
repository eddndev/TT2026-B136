use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    typed_participants::*,
};
use serde_json::Value;

use super::{inconsistent, Result};

pub(super) fn fields(value: &Value, expected: &[&str]) -> Result<()> {
    let fields = value.as_object().ok_or_else(inconsistent)?;
    if fields.len() != expected.len() || expected.iter().any(|key| !fields.contains_key(*key)) {
        return Err(inconsistent());
    }
    Ok(())
}
pub(super) fn string(value: &Value) -> Result<&str> {
    value.as_str().ok_or_else(inconsistent)
}
pub(super) fn integer(value: &Value) -> Result<u32> {
    value
        .as_u64()
        .and_then(|n| n.try_into().ok())
        .ok_or_else(inconsistent)
}
pub(super) fn text<const MAX: usize>(value: &Value) -> Result<ParticipantText<MAX>> {
    let raw = string(value)?;
    if raw.len() > MAX * 4 {
        return Err(inconsistent());
    }
    let value = ParticipantText::new(raw).map_err(|_| inconsistent())?;
    if value.as_str() != raw {
        return Err(inconsistent());
    }
    Ok(value)
}
pub(super) fn optional<const MAX: usize>(value: &Value) -> Result<Option<&str>> {
    if value.is_null() {
        return Ok(None);
    }
    text::<MAX>(value)?;
    Ok(Some(string(value)?))
}
pub(super) fn reason(value: &Value) -> Result<ParticipantReason> {
    let value = text::<500>(value)?;
    ParticipantReason::new(value.as_str()).map_err(|_| inconsistent())
}
pub(super) fn curp(value: &Value) -> Result<Curp> {
    let raw = text::<18>(value)?;
    let value = Curp::new(raw.as_str()).map_err(|_| inconsistent())?;
    if value.as_str() != raw.as_str() {
        return Err(inconsistent());
    }
    Ok(value)
}
pub(super) fn declared<T>(
    value: &Value,
    known: impl FnOnce(&Value) -> Result<T>,
) -> Result<Declared<T>> {
    if value.get("known").is_some() {
        fields(value, &["known"])?;
        Ok(Declared::Known(known(&value["known"])?))
    } else {
        fields(value, &["unknown"])?;
        Ok(Declared::Unknown(reason(&value["unknown"])?))
    }
}
pub(super) fn uuid(value: &Value) -> Result<Uuid> {
    let raw = string(value)?;
    if raw.len() != 36 {
        return Err(inconsistent());
    }
    let value = Uuid::parse_str(raw).map_err(|_| inconsistent())?;
    if value.to_string() != raw {
        return Err(inconsistent());
    }
    Ok(value)
}
pub(super) fn digest(value: &Value) -> Result<Sha256Digest> {
    let raw = string(value)?;
    if raw.len() != 64 {
        return Err(inconsistent());
    }
    let value = Sha256Digest::from_hex(raw).map_err(|_| inconsistent())?;
    if value.to_hex() != raw {
        return Err(inconsistent());
    }
    Ok(value)
}
pub(super) fn support(value: &Value) -> Result<ParticipantEvidenceLocator> {
    fields(value, &["document_id", "version", "digest", "locator"])?;
    let reference = DocumentVersionRef {
        id: DocumentId::from_uuid(uuid(&value["document_id"])?),
        version: DocumentVersion::new(integer(&value["version"])?).map_err(|_| inconsistent())?,
    };
    ParticipantEvidenceLocator::new(
        reference,
        digest(&value["digest"])?,
        text::<200>(&value["locator"])?.as_str(),
    )
    .map_err(|_| inconsistent())
}
pub(super) fn license(value: &Value) -> Result<ProfessionalLicense> {
    fields(value, &["number", "issuer"])?;
    ProfessionalLicense::new(
        text::<32>(&value["number"])?.as_str(),
        text::<200>(&value["issuer"])?.as_str(),
    )
    .map_err(|_| inconsistent())
}
