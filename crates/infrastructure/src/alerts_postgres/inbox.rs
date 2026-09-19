use super::{audit, authorization, delivery, port, records, stored, PostgresAlertStore};
use application::{alerts::*, identity::Principal, ApplicationError};
use domain::identity::{Role, UserId};
use postgres::Transaction;
use time::OffsetDateTime;

fn visible(
    tx: &mut Transaction<'_>,
    actor: &Principal,
    id: AlertId,
) -> Result<bool, ApplicationError> {
    tx.query_opt(
        "SELECT n.id FROM alert_notifications n WHERE n.id=$1 AND n.recipient=$2
         AND n.internal_enabled AND ($3 OR EXISTS(SELECT 1 FROM case_memberships m
             WHERE m.case_id=n.case_id AND m.user_id=$2))",
        &[
            &id.as_uuid(),
            &actor.id.as_uuid(),
            &(actor.role == Role::Owner),
        ],
    )
    .map(|row| row.is_some())
    .map_err(port)
}
fn checked(
    store: &PostgresAlertStore,
    tx: &mut Transaction<'_>,
    actor: &Principal,
    id: AlertId,
    now: OffsetDateTime,
) -> Result<AlertRecord, ApplicationError> {
    if !visible(tx, actor, id)? {
        return Err(AlertError::NotFound.into());
    }
    let mut record = records::load(tx, id, store.hasher.as_ref())?;
    records::current(tx, &mut record, store.hasher.as_ref(), now)?;
    record.email = delivery::status(tx, id, store.hasher.as_ref())?;
    record.validate(actor.id, now)?;
    Ok(record)
}

pub(super) fn list(
    store: &PostgresAlertStore,
    user: UserId,
    query: AlertQuery,
) -> Result<AlertPage, ApplicationError> {
    let mut client = store.client()?;
    let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
    let actor = authorization::actor(&mut tx, user)?;
    let now = store.now()?;
    let seconds = query
        .after()
        .map(|cursor| cursor.created_at().unix_timestamp());
    let nanos = query
        .after()
        .map(|cursor| cursor.created_at().nanosecond() as i32);
    let after_id = query.after().map(|cursor| cursor.id().as_uuid());
    let rows = tx
        .query(
            "SELECT n.id FROM alert_notifications n WHERE n.recipient=$1
         AND n.internal_enabled AND ($2 OR EXISTS(SELECT 1 FROM case_memberships m
             WHERE m.case_id=n.case_id AND m.user_id=$1))
         AND (NOT $3 OR n.read_seconds IS NULL)
         AND (NOT $4 OR n.resolved_seconds IS NULL)
         AND ($5::bigint IS NULL OR (n.created_seconds,n.created_nanos,n.id)<($5,$6,$7))
         ORDER BY n.created_seconds DESC,n.created_nanos DESC,n.id DESC LIMIT 100",
            &[
                &user.as_uuid(),
                &(actor.role == Role::Owner),
                &(query.read_filter() == AlertReadFilter::Unread),
                &(query.state_filter() == AlertStateFilter::Active),
                &seconds,
                &nanos,
                &after_id,
            ],
        )
        .map_err(port)?;
    let mut alerts = Vec::new();
    let mut cursor = None;
    let mut examined = 0;
    for row in &rows {
        let id = AlertId::from_uuid(row.try_get("id").map_err(stored)?);
        let record = checked(store, &mut tx, &actor, id, now)?;
        examined += 1;
        cursor = Some(AlertCursor::new(
            record.created_at,
            record.id,
            query.read_filter(),
            query.state_filter(),
        )?);
        if query.state_filter() == AlertStateFilter::All || record.state == AlertState::Active {
            alerts.push(record);
        }
        if alerts.len() == query.limit() as usize {
            break;
        }
    }
    let has_more = examined < rows.len() || rows.len() == MAX_ALERT_CANDIDATES as usize;
    let result = AlertPage {
        checked_at: now,
        alerts,
        has_more,
        next_cursor: if has_more { cursor } else { None },
    };
    result.validate(user, &query)?;
    audit(
        &mut tx,
        &actor.email,
        "alert.inbox_read",
        &format!("user:{user}"),
        now,
    )?;
    tx.commit().map_err(port)?;
    Ok(result)
}

pub(super) fn get(
    store: &PostgresAlertStore,
    user: UserId,
    id: AlertId,
) -> Result<AlertDetail, ApplicationError> {
    let mut client = store.client()?;
    let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
    let actor = authorization::actor(&mut tx, user)?;
    let now = store.now()?;
    let record = checked(store, &mut tx, &actor, id, now)?;
    audit(
        &mut tx,
        &actor.email,
        "alert.detail_read",
        &format!("alert:{id}"),
        now,
    )?;
    tx.commit().map_err(port)?;
    Ok(AlertDetail {
        checked_at: now,
        alert: record,
    })
}

pub(super) fn mark_read(
    store: &PostgresAlertStore,
    user: UserId,
    command: AlertReadCommand,
) -> Result<AlertReadReceipt, ApplicationError> {
    let mut client = store.client()?;
    let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
    let actor = authorization::actor(&mut tx, user)?;
    let now = store.now()?;
    let mut record = checked(store, &mut tx, &actor, command.alert_id, now)?;
    if let Some(row) = tx.query_opt("SELECT recipient,alert_id,read_seconds,read_nanos FROM alert_read_receipts WHERE operation_id=$1", &[&command.operation_id.as_uuid()]).map_err(port)? {
        let recipient: uuid::Uuid = row.try_get("recipient").map_err(stored)?;
        let alert: uuid::Uuid = row.try_get("alert_id").map_err(stored)?;
        if recipient != user.as_uuid() || alert != command.alert_id.as_uuid() {
            return Err(AlertError::OperationConflict.into());
        }
        let original = super::codec::row_time(&row, "read_seconds", "read_nanos")?.ok_or_else(||stored("missing read receipt timestamp"))?;
        if record.read_at != Some(original) { return Err(stored("read receipt differs from notification")); }
    } else {
        let read_at = record.read_at.unwrap_or(now);
        tx.execute("UPDATE alert_notifications SET read_seconds=$2,read_nanos=$3 WHERE id=$1 AND read_seconds IS NULL", &[&command.alert_id.as_uuid(), &read_at.unix_timestamp(), &(read_at.nanosecond() as i32)]).map_err(port)?;
        tx.execute("INSERT INTO alert_read_receipts(operation_id,recipient,alert_id,read_seconds,read_nanos) VALUES($1,$2,$3,$4,$5)",
            &[&command.operation_id.as_uuid(),&user.as_uuid(),&command.alert_id.as_uuid(),&read_at.unix_timestamp(),&(read_at.nanosecond() as i32)]).map_err(port)?;
        record.read_at = Some(read_at);
    }
    record.validate(user, now)?;
    audit(
        &mut tx,
        &actor.email,
        "alert.read_recorded",
        &format!(
            "alert:{}:operation:{}",
            command.alert_id, command.operation_id
        ),
        now,
    )?;
    tx.commit().map_err(port)?;
    Ok(AlertReadReceipt {
        operation_id: command.operation_id,
        checked_at: now,
        alert: record,
    })
}
