use super::{inconsistent, port};
use application::{
    deadline_dispatch::DeadlineEventDispatchPosition, deadline_reevaluation::*, ApplicationError,
};
use domain::cases::CaseId;
use postgres::{Row, Transaction};

pub(super) fn load(
    tx: &mut Transaction<'_>,
    position: DeadlineEventDispatchPosition,
) -> Result<Option<SourceEventReference>, ApplicationError> {
    let completed = position
        .completed_sequence
        .map(i64::try_from)
        .transpose()
        .map_err(inconsistent)?;
    let row = tx.query_opt("SELECT sequence,source_kind,source_id,revision,case_id,hearing_id,operation_id
        FROM deadline_source_events WHERE sequence>coalesce($1::bigint,0) ORDER BY sequence LIMIT 1",
        &[&completed]).map_err(port)?;
    let event = row.as_ref().map(decode).transpose()?;
    if position.active_sequence.is_some()
        && position.active_sequence != event.map(|event| event.sequence)
    {
        return Err(inconsistent(
            "active dispatch event is not the next stored event",
        ));
    }
    Ok(event)
}

fn decode(row: &Row) -> Result<SourceEventReference, ApplicationError> {
    let sequence: i64 = row.try_get(0).map_err(inconsistent)?;
    let revision: i64 = row.try_get(3).map_err(inconsistent)?;
    if sequence <= 0 || revision <= 0 {
        return Err(inconsistent("invalid dispatch event sequence or revision"));
    }
    let family = match row.try_get::<_, &str>(1).map_err(inconsistent)? {
        "resolution" => DependencyFamily::Resolution,
        "notification" => DependencyFamily::Notification,
        "hearing_result" => DependencyFamily::HearingResult,
        "calendar" => DependencyFamily::Calendar,
        "profile" => DependencyFamily::Profile,
        _ => return Err(inconsistent("unknown dispatch event family")),
    };
    let event = SourceEventReference {
        sequence: u64::try_from(sequence).map_err(inconsistent)?,
        family,
        source_id: row.try_get(2).map_err(inconsistent)?,
        revision: u32::try_from(revision).map_err(inconsistent)?,
        case_id: row
            .try_get::<_, Option<uuid::Uuid>>(4)
            .map_err(inconsistent)?
            .map(CaseId::from_uuid),
        hearing_id: row.try_get(5).map_err(inconsistent)?,
        operation_id: row.try_get(6).map_err(inconsistent)?,
    };
    let valid = match family {
        DependencyFamily::Resolution | DependencyFamily::Notification => {
            event.case_id.is_some() && event.hearing_id.is_none()
        }
        DependencyFamily::HearingResult => event.case_id.is_some() && event.hearing_id.is_some(),
        DependencyFamily::Calendar => event.case_id.is_none() && event.hearing_id.is_none(),
        DependencyFamily::Profile => event.hearing_id.is_none(),
    };
    if !valid {
        return Err(inconsistent("dispatch event scope is invalid"));
    }
    Ok(event)
}
