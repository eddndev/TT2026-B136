use super::port;
use application::{procedural_resources::ResourceId, ApplicationError};
use domain::cases::CaseId;
use postgres::Client;
use uuid::Uuid;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let broken: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_procedural_resources p
            LEFT JOIN case_procedural_resource_revisions r
                ON r.resource_id=p.id AND r.case_id=p.case_id AND r.revision=1
            WHERE r.resource_id IS NULL OR p.initial_revision<>1 OR r.action<>'register')
        OR EXISTS(SELECT 1 FROM case_procedural_resource_revisions r
            LEFT JOIN case_procedural_resources p ON p.id=r.resource_id AND p.case_id=r.case_id
            WHERE p.id IS NULL OR r.values_digest<>pg_catalog.sha256(r.values_canonical)
                OR r.sources_digest<>pg_catalog.sha256(r.sources_canonical)
                OR r.submission_digest<>pg_catalog.sha256(r.submission_canonical)
                OR r.capture_digest<>pg_catalog.sha256(r.capture_canonical))
        OR EXISTS(SELECT 1 FROM case_procedural_resource_revisions GROUP BY resource_id
            HAVING min(revision)<>1 OR max(revision)<>count(*))
        OR EXISTS(SELECT 1 FROM case_procedural_resource_acts a
            LEFT JOIN case_procedural_resource_revisions r ON r.resource_id=a.resource_id
                AND r.case_id=a.case_id AND r.revision=a.initial_resource_revision
            WHERE r.act_id IS DISTINCT FROM a.id OR r.act_revision IS DISTINCT FROM 1
                OR r.action IS DISTINCT FROM 'record_act')
        OR EXISTS(SELECT 1 FROM case_procedural_resource_revisions r
            LEFT JOIN case_procedural_resource_acts a ON a.id=r.act_id
                AND a.resource_id=r.resource_id AND a.case_id=r.case_id
            WHERE r.act_id IS NOT NULL AND a.id IS NULL)
        OR EXISTS(SELECT 1 FROM case_procedural_resource_revisions WHERE act_id IS NOT NULL
            GROUP BY act_id HAVING min(act_revision)<>1 OR max(act_revision)<>count(*))
        OR EXISTS(SELECT 1 FROM (
            SELECT act_previous_resource_revision,
                lag(revision) OVER(PARTITION BY act_id ORDER BY act_revision) AS previous
            FROM case_procedural_resource_revisions WHERE act_id IS NOT NULL) a
            WHERE a.act_previous_resource_revision IS DISTINCT FROM a.previous)",
            &[],
        )
        .map_err(port)?
        .get(0);
    if broken {
        return Err(inconsistent());
    }
    let mut id: Option<Uuid> = None;
    let mut revision = 0_i64;
    loop {
        let rows = tx
            .query(
                "SELECT resource_id,revision,case_id FROM case_procedural_resource_revisions
            WHERE $1::uuid IS NULL OR (resource_id,revision)>($1,$2)
            ORDER BY resource_id,revision LIMIT 64",
                &[&id, &revision],
            )
            .map_err(port)?;
        for row in &rows {
            let current_id: Uuid = row.try_get(0).map_err(|_| inconsistent())?;
            let current_revision: i64 = row.try_get(1).map_err(|_| inconsistent())?;
            let case = CaseId::from_uuid(row.try_get(2).map_err(|_| inconsistent())?);
            let selected = crate::procedural_resource_postgres::storage::revision(current_revision)
                .map_err(|_| inconsistent())?;
            crate::procedural_resource_postgres::storage::detail(
                &mut tx,
                case,
                ResourceId::from_uuid(current_id),
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
        "procedural resource inventory is inconsistent; restore a consistent database".into(),
    )
}
