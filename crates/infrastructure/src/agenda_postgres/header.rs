use super::inconsistent;
use application::{agenda::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime};
use postgres::Row;
use uuid::Uuid;

pub(super) struct Header {
    pub case: CaseId,
    pub revision: u32,
    pub key: AgendaCursor,
    pub status: String,
}

impl Header {
    pub fn decode(row: &Row) -> Result<Self, ApplicationError> {
        let kind = match row.try_get::<_, i16>("kind_rank").map_err(inconsistent)? {
            0 => AgendaItemKind::Hearing,
            1 => AgendaItemKind::Deadline,
            _ => return Err(inconsistent("unknown agenda candidate kind")),
        };
        let seconds: i64 = row.try_get("seconds").map_err(inconsistent)?;
        let nanos: i32 = row.try_get("nanoseconds").map_err(inconsistent)?;
        let at = OffsetDateTime::from_unix_timestamp(seconds)
            .map_err(inconsistent)?
            .replace_nanosecond(u32::try_from(nanos).map_err(inconsistent)?)
            .map_err(inconsistent)?;
        let id: Uuid = row.try_get("id").map_err(inconsistent)?;
        Ok(Self {
            case: CaseId::from_uuid(row.try_get("case_id").map_err(inconsistent)?),
            revision: u32::try_from(row.try_get::<_, i64>("revision").map_err(inconsistent)?)
                .map_err(inconsistent)?,
            key: AgendaCursor::new(at, kind, id).map_err(inconsistent)?,
            status: row.try_get("status").map_err(inconsistent)?,
        })
    }
}

pub(super) const fn kind_rank(kind: AgendaItemKind) -> i16 {
    match kind {
        AgendaItemKind::Hearing => 0,
        AgendaItemKind::Deadline => 1,
    }
}
