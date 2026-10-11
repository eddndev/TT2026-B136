use application::{identity::invitations::*, ApplicationError};
use domain::{
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
use postgres::{GenericClient, Row, Transaction};
use serde_json::{json, Value};
use std::str::FromStr;

pub(super) fn storage() -> ApplicationError {
    ApplicationError::Port("invitation storage operation failed".into())
}
pub(super) fn invalid() -> ApplicationError {
    ApplicationError::InvalidCredentials
}
pub(super) fn seconds(at: i64) -> Result<OffsetDateTime, ApplicationError> {
    OffsetDateTime::from_unix_timestamp(at).map_err(|_| storage())
}
pub(super) fn now(tx: &mut Transaction<'_>) -> Result<i64, ApplicationError> {
    tx.query_one(
        "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
        &[],
    )
    .map_err(|_| storage())
    .map(|row| row.get(0))
}
pub(super) fn authority<C: GenericClient>(
    tx: &mut C,
    actor: &InvitationActor,
) -> Result<(), ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT email,role,active,revision,auth_generation FROM users WHERE id=$1 FOR SHARE",
            &[&actor.principal.id.as_uuid()],
        )
        .map_err(|_| storage())?
        .ok_or(ApplicationError::InvalidSession)?;
    if actor.principal.role != Role::Owner
        || row.get::<_, String>("role") != "owner"
        || !row.get::<_, bool>("active")
    {
        return Err(ApplicationError::PermissionDenied);
    }
    if row.get::<_, String>("email") != actor.principal.email
        || u64::try_from(row.get::<_, i64>("revision")).ok() != Some(actor.revision)
        || u64::try_from(row.get::<_, i64>("auth_generation")).ok() != Some(actor.auth_generation)
    {
        return Err(ApplicationError::InvalidSession);
    }
    Ok(())
}
pub(super) fn account_absent<C: GenericClient>(
    tx: &mut C,
    email: &str,
) -> Result<(), ApplicationError> {
    if tx
        .query_opt("SELECT id FROM users WHERE email=$1", &[&email])
        .map_err(|_| storage())?
        .is_some()
    {
        return Err(ApplicationError::UserAlreadyExists);
    }
    Ok(())
}
pub(super) fn summary(row: &Row, at: i64) -> Result<InvitationSummary, ApplicationError> {
    let accepted: Option<i64> = row.get("accepted_at");
    let expires: i64 = row.get("expires_at");
    Ok(InvitationSummary {
        id: InvitationId::from_uuid(row.get("id")),
        revision: u64::try_from(row.get::<_, i64>("revision")).map_err(|_| storage())?,
        operation_id: row.get("operation_id"),
        email: row.get("email"),
        role: Role::from_str(&row.get::<_, String>("role"))?,
        status: if accepted.is_some() {
            InvitationStatus::Accepted
        } else if row.get::<_, Option<i64>>("cancelled_at").is_some() || expires <= at {
            InvitationStatus::Expired
        } else if row.get::<_, Option<Vec<u8>>>("pending_material").is_some() {
            InvitationStatus::PendingMfa
        } else {
            InvitationStatus::Pending
        },
        delivery: delivery(&row.get::<_, String>("delivery_state"))?,
        issued_at: seconds(row.get("issued_at"))?,
        expires_at: seconds(expires)?,
        accepted_at: accepted.map(seconds).transpose()?,
    })
}
pub(super) fn delivery(value: &str) -> Result<InvitationDeliveryState, ApplicationError> {
    Ok(match value {
        "pending" => InvitationDeliveryState::Pending,
        "sent" => InvitationDeliveryState::Sent,
        "uncertain" => InvitationDeliveryState::Uncertain,
        "rejected" => InvitationDeliveryState::Rejected,
        _ => return Err(storage()),
    })
}
pub(super) fn pending(row: &Row) -> Result<Option<PendingInvitation>, ApplicationError> {
    let Some(material) = row.get::<_, Option<Vec<u8>>>("pending_material") else {
        return Ok(None);
    };
    Ok(Some(PendingInvitation {
        operation_id: row.get("pending_operation"),
        request_digest: digest(row.get("pending_digest"))?,
        protected_material: material,
    }))
}
pub(super) fn digest(bytes: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Ok(Sha256Digest::from_array(
        bytes.try_into().map_err(|_| storage())?,
    ))
}
pub(super) fn encode_summary(s: &InvitationSummary) -> Value {
    json!({"id":s.id.as_uuid(),"revision":s.revision,"operation_id":s.operation_id,"email":s.email,"role":s.role.as_str(),"issued_at":s.issued_at.unix_timestamp(),"expires_at":s.expires_at.unix_timestamp()})
}
pub(super) fn decode_summary(v: Value) -> Result<InvitationSummary, ApplicationError> {
    Ok(InvitationSummary {
        id: InvitationId::from_uuid(uuid(&v, "id")?),
        revision: v["revision"].as_u64().ok_or_else(storage)?,
        operation_id: uuid(&v, "operation_id")?,
        email: string(&v, "email")?,
        role: Role::from_str(&string(&v, "role")?)?,
        status: InvitationStatus::Pending,
        delivery: InvitationDeliveryState::Pending,
        issued_at: seconds(integer(&v, "issued_at")?)?,
        expires_at: seconds(integer(&v, "expires_at")?)?,
        accepted_at: None,
    })
}
pub(super) fn encode_receipt(r: &InvitationReceipt) -> Value {
    json!({"id":r.id.as_uuid(),"revision":r.revision,"operation_id":r.operation_id,"user_id":r.user_id.as_uuid(),"email":r.email,"role":r.role.as_str(),"accepted_at":r.accepted_at.unix_timestamp()})
}
pub(super) fn decode_receipt(v: Value) -> Result<InvitationReceipt, ApplicationError> {
    Ok(InvitationReceipt {
        id: InvitationId::from_uuid(uuid(&v, "id")?),
        revision: v["revision"].as_u64().ok_or_else(storage)?,
        operation_id: uuid(&v, "operation_id")?,
        user_id: UserId::from_uuid(uuid(&v, "user_id")?),
        email: string(&v, "email")?,
        role: Role::from_str(&string(&v, "role")?)?,
        accepted_at: seconds(integer(&v, "accepted_at")?)?,
    })
}
fn uuid(v: &Value, key: &str) -> Result<Uuid, ApplicationError> {
    Uuid::parse_str(v[key].as_str().ok_or_else(storage)?).map_err(|_| storage())
}
fn string(v: &Value, key: &str) -> Result<String, ApplicationError> {
    v[key].as_str().map(str::to_owned).ok_or_else(storage)
}
fn integer(v: &Value, key: &str) -> Result<i64, ApplicationError> {
    v[key].as_i64().ok_or_else(storage)
}
