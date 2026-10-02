use super::{capacity, port, MAX_CASES};
use application::{identity::Principal, ApplicationError};
use domain::{
    case_administration::CaseAdministrativeStatus, cases::CaseId, crypto::DocumentHasher,
    identity::Role,
};
use postgres::Transaction;
use uuid::Uuid;

pub(super) struct Cases {
    pub all: Vec<Uuid>,
    pub active: Vec<Uuid>,
}
pub(super) fn load(
    tx: &mut Transaction<'_>,
    actor: &Principal,
    hasher: &dyn DocumentHasher,
) -> Result<Cases, ApplicationError> {
    let rows = if actor.role == Role::Owner {
        tx.query("SELECT id FROM cases ORDER BY id LIMIT $1", &[&((MAX_CASES + 1) as i64)])
    } else {
        tx.query("SELECT case_id AS id FROM case_memberships WHERE user_id=$1 ORDER BY case_id LIMIT $2 FOR SHARE", &[&actor.id.as_uuid(), &((MAX_CASES + 1) as i64)])
    }.map_err(port)?;
    capacity(rows.len(), MAX_CASES, "case")?;
    let mut result = Cases {
        all: Vec::with_capacity(rows.len()),
        active: Vec::new(),
    };
    for row in rows {
        let id: Uuid = row.try_get("id").map_err(port)?;
        let detail = crate::cases::storage::detail(tx, CaseId::from_uuid(id), hasher)?;
        result.all.push(id);
        if detail.administration.values().status() == CaseAdministrativeStatus::Active {
            result.active.push(id);
        }
    }
    Ok(result)
}
