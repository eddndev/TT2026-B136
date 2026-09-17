//! Strict reconstruction of stored procedural facts and exact source projections.
mod declarations;
mod helpers;
mod provenance;
mod source_entries;
mod sources;
mod temporal;
mod values;

use application::{procedural_facts::*, ApplicationError};
use serde_json::Value;
type Result<T> = std::result::Result<T, ApplicationError>;

/// Reconstructs normalized values and requires their exact stored canonical bytes.
pub fn values(family: &str, canonical: &[u8], projection: &Value) -> Result<ProceduralFactValues> {
    let result = match family {
        "resolution"
            if (27..=17700).contains(&canonical.len()) && canonical.starts_with(b"PFRES1") =>
        {
            ProceduralFactValues::Resolution(Box::new(values::resolution(projection)?))
        }
        "notification"
            if (67..=58671).contains(&canonical.len()) && canonical.starts_with(b"PFNOT1") =>
        {
            ProceduralFactValues::Notification(Box::new(values::notification(projection)?))
        }
        _ => return Err(inconsistent()),
    };
    let encoded = match &result {
        ProceduralFactValues::Resolution(value) => value.canonical_bytes(),
        ProceduralFactValues::Notification(value) => value.canonical_bytes(),
    };
    if encoded != canonical {
        return Err(inconsistent());
    }
    Ok(result)
}

/// Rebuilds captured source views without resolving heads or admitting documents.
pub fn sources(canonical: &[u8], projection: &Value) -> Result<FactSources> {
    if !(19..=36847).contains(&canonical.len()) || !canonical.starts_with(b"PFSRC1") {
        return Err(inconsistent());
    }
    let result = sources::decode(projection)?;
    if fact_sources_bytes(&result).map_err(|_| inconsistent())? != canonical {
        return Err(inconsistent());
    }
    Ok(result)
}
fn inconsistent() -> ApplicationError {
    ProceduralFactError::StoredInconsistent("canonical fact or projection is inconsistent".into())
        .into()
}

/// Reuse the strict normalized procedural-time projection for stored declarations.
pub(crate) fn declared_time(
    projection: &Value,
) -> Result<domain::procedural_time::DeclaredProceduralTime> {
    temporal::value_time(projection)
}
