use super::{audit, authorization, codec, port, stored, PostgresAlertStore};
use application::{alerts::*, ApplicationError};
use domain::{
    alerts::{AlertAnticipations, AlertLeadHours},
    crypto::DocumentHasher,
    identity::UserId,
};
use postgres::{Row, Transaction};
use serde_json::{json, Value};

pub(super) fn channels(value: AlertChannels) -> Value {
    json!([value.internal, value.email])
}
pub(super) fn read_channels(value: &Value) -> Result<AlertChannels, ApplicationError> {
    Ok(AlertChannels {
        internal: codec::boolean(&value[0])?,
        email: codec::boolean(&value[1])?,
    })
}
fn family(value: &AlertFamilyPreferences) -> Value {
    json!({"hours":value.anticipations.hours().iter().map(|h|h.get()).collect::<Vec<_>>(),"channels":channels(value.channels)})
}
fn read_family(value: &Value) -> Result<AlertFamilyPreferences, ApplicationError> {
    let hours = value["hours"]
        .as_array()
        .ok_or_else(|| stored("invalid alert lead list"))?
        .iter()
        .map(|v| {
            AlertLeadHours::new(u16::try_from(codec::integer(v)?).map_err(stored)?).map_err(stored)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AlertFamilyPreferences {
        anticipations: AlertAnticipations::new(hours).map_err(stored)?,
        channels: read_channels(&value["channels"])?,
    })
}
pub(super) fn values(value: &AlertPreferenceValues) -> Value {
    json!({"hearing":family(&value.hearing_upcoming),"deadline":family(&value.deadline_upcoming),
        "overdue":channels(value.overdue_unattended),"review":channels(value.review_required),"changed":channels(value.due_changed_soon)})
}
pub(super) fn read_values(value: &Value) -> Result<AlertPreferenceValues, ApplicationError> {
    let result = AlertPreferenceValues {
        hearing_upcoming: read_family(&value["hearing"])?,
        deadline_upcoming: read_family(&value["deadline"])?,
        overdue_unattended: read_channels(&value["overdue"])?,
        review_required: read_channels(&value["review"])?,
        due_changed_soon: read_channels(&value["changed"])?,
    };
    if values(&result) != *value {
        return Err(stored("alert preference payload differs"));
    }
    Ok(result)
}
pub(super) fn decode(
    row: &Row,
    hasher: &dyn DocumentHasher,
    transport: AlertEmailTransport,
) -> Result<AlertPreferences, ApplicationError> {
    let revision =
        u32::try_from(row.try_get::<_, i64>("revision").map_err(stored)?).map_err(stored)?;
    let result = AlertPreferences {
        user_id: UserId::from_uuid(row.try_get("user_id").map_err(stored)?),
        revision,
        values: read_values(&codec::read(row, hasher)?)?,
        updated_at: codec::row_time(row, "recorded_seconds", "recorded_nanos")?,
        receipt: Some(AlertPreferenceReceipt {
            operation_id: AlertOperationId::from_uuid(row.try_get("operation_id").map_err(stored)?),
            expected_revision: revision
                .checked_sub(1)
                .ok_or_else(|| stored("zero preference revision"))?,
        }),
        email_transport: transport,
    };
    result.validate(result.user_id)?;
    Ok(result)
}
pub(super) fn load(
    tx: &mut Transaction<'_>,
    user: UserId,
    hasher: &dyn DocumentHasher,
    transport: AlertEmailTransport,
) -> Result<AlertPreferences, ApplicationError> {
    match tx
        .query_opt(
            "SELECT * FROM alert_preferences WHERE user_id=$1 ORDER BY revision DESC LIMIT 1",
            &[&user.as_uuid()],
        )
        .map_err(port)?
    {
        Some(row) => decode(&row, hasher, transport),
        None => Ok(AlertPreferences::initial(user, transport)),
    }
}
pub(super) fn get(
    store: &PostgresAlertStore,
    user: UserId,
) -> Result<AlertPreferences, ApplicationError> {
    let mut client = store.client()?;
    let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
    let actor = authorization::actor(&mut tx, user)?;
    let now = store.now()?;
    let result = load(&mut tx, user, store.hasher.as_ref(), store.transport())?;
    audit(
        &mut tx,
        &actor.email,
        "alert.preferences_read",
        &format!("user:{user}"),
        now,
    )?;
    tx.commit().map_err(port)?;
    Ok(result)
}
pub(super) fn save(
    store: &PostgresAlertStore,
    user: UserId,
    command: AlertPreferenceCommand,
) -> Result<AlertPreferences, ApplicationError> {
    let mut client = store.client()?;
    let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
    let actor = authorization::actor(&mut tx, user)?;
    let now = store.now()?;
    if let Some(row) = tx
        .query_opt(
            "SELECT * FROM alert_preferences WHERE operation_id=$1",
            &[&command.operation_id.as_uuid()],
        )
        .map_err(port)?
    {
        let saved = decode(&row, store.hasher.as_ref(), store.transport())?;
        if saved.user_id != user
            || saved.values != command.values
            || saved.receipt.map(|r| r.expected_revision) != Some(command.expected_revision)
        {
            return Err(AlertError::OperationConflict.into());
        }
        audit(
            &mut tx,
            &actor.email,
            "alert.preferences_recovered",
            &format!("user:{user}:operation:{}", command.operation_id),
            now,
        )?;
        tx.commit().map_err(port)?;
        return Ok(saved);
    }
    let previous = load(&mut tx, user, store.hasher.as_ref(), store.transport())?;
    if previous.revision != command.expected_revision {
        return Err(AlertError::RevisionConflict.into());
    }
    let revision = command
        .expected_revision
        .checked_add(1)
        .ok_or(AlertError::RevisionConflict)?;
    let canonical = values(&command.values);
    read_values(&canonical)?;
    let (bytes, digest) = codec::payload(&canonical, store.hasher.as_ref())?;
    tx.execute("INSERT INTO alert_preferences(user_id,revision,operation_id,recorded_seconds,recorded_nanos,payload,payload_digest)
        VALUES($1,$2,$3,$4,$5,$6,$7)",&[&user.as_uuid(),&i64::from(revision),&command.operation_id.as_uuid(),&now.unix_timestamp(),&(now.nanosecond() as i32),&bytes,&digest]).map_err(port)?;
    // Pending plans are reconsidered before activation; completed occurrences remain unique.
    tx.execute(
        "UPDATE alert_scan_cursor SET kind=0,id=NULL,active_kind=NULL,active_id=NULL,
        after_recipient=NULL,next_seconds=NULL,next_nanos=NULL WHERE singleton",
        &[],
    )
    .map_err(port)?;
    let result = load(&mut tx, user, store.hasher.as_ref(), store.transport())?;
    result.validate_command(user, &command)?;
    audit(
        &mut tx,
        &actor.email,
        "alert.preferences_saved",
        &format!(
            "user:{user}:revision:{revision}:operation:{}",
            command.operation_id
        ),
        now,
    )?;
    tx.commit().map_err(port)?;
    Ok(result)
}
