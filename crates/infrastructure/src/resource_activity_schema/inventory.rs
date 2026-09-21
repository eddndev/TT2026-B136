use super::port;
use application::{
    resource_activities::{ResourceActivityId, ResourceId},
    ApplicationError,
};
use domain::cases::CaseId;
use postgres::Client;
use uuid::Uuid;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let broken: bool = tx.query_one("SELECT EXISTS(
        SELECT 1 FROM case_resource_activity_associations p LEFT JOIN case_resource_activity_association_revisions r
            ON r.association_id=p.id AND r.case_id=p.case_id AND r.resource_id=p.resource_id AND r.revision=p.initial_revision
        WHERE p.initial_revision<>1 OR r.association_id IS NULL OR r.action<>'link' OR r.status<>'linked')
        OR EXISTS(SELECT 1 FROM case_resource_activity_association_revisions r LEFT JOIN case_resource_activity_associations p
            ON p.id=r.association_id AND p.case_id=r.case_id AND p.resource_id=r.resource_id
        WHERE p.id IS NULL OR r.submission_digest<>pg_catalog.sha256(r.submission_canonical)
            OR r.capture_digest<>pg_catalog.sha256(r.capture_canonical))
        OR EXISTS(SELECT 1 FROM case_resource_activity_association_revisions GROUP BY association_id
            HAVING min(revision)<>1 OR max(revision)<>count(*) OR max(revision)>2)", &[]).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent());
    }
    let mut id: Option<Uuid> = None;
    let mut revision = 0_i64;
    loop {
        let rows = tx.query("SELECT association_id,revision,case_id,resource_id FROM case_resource_activity_association_revisions
            WHERE $1::uuid IS NULL OR (association_id,revision)>($1,$2)
            ORDER BY association_id,revision LIMIT 64", &[&id,&revision]).map_err(port)?;
        for row in &rows {
            let current_id: Uuid = row.try_get("association_id").map_err(|_| inconsistent())?;
            let current_revision: i64 = row.try_get("revision").map_err(|_| inconsistent())?;
            let case = CaseId::from_uuid(row.try_get("case_id").map_err(|_| inconsistent())?);
            let resource =
                ResourceId::from_uuid(row.try_get("resource_id").map_err(|_| inconsistent())?);
            let selected = crate::resource_activity_postgres::storage::revision(current_revision)
                .map_err(|_| inconsistent())?;
            crate::resource_activity_postgres::storage::detail(
                &mut tx,
                case,
                resource,
                ResourceActivityId::from_uuid(current_id),
                Some(selected),
                &crate::RingSha256Hasher,
            )
            .map_err(|_| inconsistent())?;
            id = Some(current_id);
            revision = current_revision;
        }
        if rows.len() < 64 {
            return tx.rollback().map_err(port);
        }
    }
}
fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "resource activity inventory is inconsistent; restore a consistent database".into(),
    )
}
