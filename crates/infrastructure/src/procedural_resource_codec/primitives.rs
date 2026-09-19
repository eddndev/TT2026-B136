use super::{invalid, Result};
use application::{
    case_stages::StageSupportSnapshot,
    documents::{StageDocumentFormat, StageFormatPolicy},
    procedural_facts::*,
};
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime},
};
use serde_json::{json, Value};
use uuid::Uuid;

pub(super) fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or_else(invalid)
}
pub(super) fn number(v: &Value) -> Result<u32> {
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(invalid)
}
pub(super) fn uuid(v: &Value) -> Result<Uuid> {
    Uuid::parse_str(string(v)?).map_err(|_| invalid())
}
pub(super) fn digest(v: &Value) -> Result<Sha256Digest> {
    Sha256Digest::from_hex(string(v)?).map_err(|_| invalid())
}
pub(super) fn label(v: &Value) -> Result<FactLabel> {
    FactLabel::new(string(v)?).map_err(|_| invalid())
}
pub(super) fn text(v: &Value) -> Result<FactText> {
    FactText::new(string(v)?).map_err(|_| invalid())
}
pub(super) fn optional<T>(
    v: &Value,
    decode: impl FnOnce(&Value) -> Result<T>,
) -> Result<Option<T>> {
    if v.is_null() {
        Ok(None)
    } else {
        decode(v).map(Some)
    }
}
pub(super) fn array(v: &Value, bound: usize) -> Result<&[Value]> {
    v.as_array()
        .filter(|a| a.len() <= bound)
        .map(Vec::as_slice)
        .ok_or_else(invalid)
}
pub(super) fn declaration<T>(v: &FactDeclaration<T>, encode: impl FnOnce(&T) -> Value) -> Value {
    match v {
        FactDeclaration::Known(v) => json!({"known":encode(v)}),
        FactDeclaration::Unknown(v) => json!({"unknown":v.as_str()}),
    }
}
pub(super) fn declared<T>(
    v: &Value,
    decode: impl FnOnce(&Value) -> Result<T>,
) -> Result<FactDeclaration<T>> {
    if let Some(v) = v.get("known") {
        Ok(FactDeclaration::Known(decode(v)?))
    } else {
        Ok(FactDeclaration::Unknown(text(&v["unknown"])?))
    }
}
pub(super) fn time(v: DeclaredProceduralTime) -> Value {
    let Some(date) = v.local_date() else {
        return json!({"precision":"unknown"});
    };
    let precision = match v.precision() {
        DeclaredProceduralPrecision::Date => "date",
        DeclaredProceduralPrecision::Minute => "minute",
        DeclaredProceduralPrecision::Second => "second",
        DeclaredProceduralPrecision::Unknown => "unknown",
    };
    let mut result = json!({"precision":precision,"year":date.date().year(),"month":u8::from(date.date().month()),"day":date.date().day(),"offset_seconds":v.offset().map(|o|o.whole_seconds())});
    if let Some(hour) = v.local_hour() {
        result["hour"] = json!(hour);
        result["minute"] = json!(v.local_minute());
    }
    if let Some(second) = v.local_second() {
        result["second"] = json!(second);
    }
    result
}
pub(super) fn reference(v: &Value) -> Result<DocumentVersionRef> {
    Ok(DocumentVersionRef {
        id: DocumentId::from_uuid(uuid(&v["id"])?),
        version: DocumentVersion::new(number(&v["version"])?).map_err(|_| invalid())?,
    })
}
pub(super) fn evidence(v: &FactEvidence) -> Value {
    json!({"id":v.reference().id.as_uuid().to_string(),"version":v.reference().version.get(),"digest":v.digest().to_hex(),"locator":v.locator().as_str()})
}
pub(super) fn decode_evidence(v: &Value) -> Result<FactEvidence> {
    Ok(FactEvidence::new(
        reference(v)?,
        digest(&v["digest"])?,
        label(&v["locator"])?,
    ))
}
pub(crate) fn encode_supports(values: &[StageSupportSnapshot]) -> Value {
    json!(values.iter().map(|v| json!({"id":v.reference.id.as_uuid().to_string(),"version":v.reference.version.get(),"digest":v.digest.to_hex(),"name":v.name,"format":match v.format {StageDocumentFormat::Pdf=>"pdf",StageDocumentFormat::Docx=>"docx"},"policy":"pdf_docx_v1"})).collect::<Vec<_>>())
}
pub(crate) fn decode_supports(v: &Value) -> Result<Vec<StageSupportSnapshot>> {
    let values = array(v, 2)?
        .iter()
        .map(|item| {
            Ok(StageSupportSnapshot {
                reference: reference(item)?,
                digest: digest(&item["digest"])?,
                name: string(&item["name"])?.into(),
                format: match string(&item["format"])? {
                    "pdf" => StageDocumentFormat::Pdf,
                    "docx" => StageDocumentFormat::Docx,
                    _ => return Err(invalid()),
                },
                policy: StageFormatPolicy::PdfDocxV1,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if encode_supports(&values) != *v {
        return Err(invalid());
    }
    Ok(values)
}
