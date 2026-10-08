//! Strict correction values; decoding does not prove authorization or faithful transcription.
use application::{measure_corrections::MeasureAdministrativeError, ApplicationError};
use domain::{hearings::HearingNote, precautionary_measures::*};
use serde_json::{json, Value};

type Result<T> = std::result::Result<T, ApplicationError>;

fn inconsistent() -> ApplicationError {
    MeasureAdministrativeError::StoredInconsistent(
        "stored correction values are inconsistent".into(),
    )
    .into()
}

fn fields(value: &Value, names: &[&str]) -> Result<()> {
    let object = value.as_object().ok_or_else(inconsistent)?;
    if object.len() != names.len() || names.iter().any(|name| !object.contains_key(*name)) {
        return Err(inconsistent());
    }
    Ok(())
}

fn note(value: &Value) -> Result<HearingNote> {
    let raw = value.as_str().ok_or_else(inconsistent)?;
    if raw.len() > 4000 {
        return Err(inconsistent());
    }
    let note = HearingNote::new(raw).map_err(|_| inconsistent())?;
    if note.as_str() != raw {
        return Err(inconsistent());
    }
    Ok(note)
}

pub fn values(bytes: &[u8], projection: &Value) -> Result<MeasureCorrectionValues> {
    if !(38..=20040).contains(&bytes.len()) || !bytes.starts_with(b"MCVAL1") {
        return Err(inconsistent());
    }
    fields(projection, &["conditions", "validity", "supervision_text"])?;
    let validity = &projection["validity"];
    fields(validity, &["start", "statement", "end"])?;
    let temporal = crate::measure_decision_codec::temporal::decode;
    let start = temporal(&validity["start"]).map_err(|_| inconsistent())?;
    let end = if validity["end"].is_null() {
        None
    } else {
        Some(temporal(&validity["end"]).map_err(|_| inconsistent())?)
    };
    let value = MeasureCorrectionValues::new(
        note(&projection["conditions"])?,
        MeasureValidity::new(start, note(&validity["statement"])?, end)
            .map_err(|_| inconsistent())?,
        note(&projection["supervision_text"])?,
    );
    if value.canonical_bytes() != bytes {
        return Err(inconsistent());
    }
    Ok(value)
}

pub fn view(value: &MeasureCorrectionValues) -> Value {
    let validity = value.validity();
    let temporal = crate::measure_decision_codec::temporal::view;
    json!({"conditions":value.conditions().as_str(),"validity":{
        "start":temporal(validity.start()),"statement":validity.statement().as_str(),
        "end":validity.end().map(temporal)},"supervision_text":value.supervision_text().as_str()})
}
