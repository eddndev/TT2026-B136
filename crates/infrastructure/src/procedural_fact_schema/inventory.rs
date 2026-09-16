use super::port;
use application::{procedural_facts::*, ApplicationError};
use domain::cases::CaseId;
use postgres::Client;
use uuid::Uuid;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let broken: bool = tx.query_one(
        "SELECT
         EXISTS(SELECT 1 FROM case_procedural_facts f
            LEFT JOIN case_procedural_fact_revisions r ON r.family=f.family AND r.id=f.id AND r.revision=1
            WHERE r.id IS NULL OR f.initial_revision<>1 OR r.case_id<>f.case_id
                OR f.family NOT IN ('resolution','notification')
                OR (f.family='resolution' AND f.parent_resolution_id IS NOT NULL)
                OR (f.family='notification' AND f.parent_resolution_id IS NULL))
         OR EXISTS(SELECT 1 FROM case_procedural_fact_revisions r
            LEFT JOIN case_procedural_facts f USING(family,id,case_id)
            WHERE f.id IS NULL OR r.revision NOT BETWEEN 1 AND 4294967295
                OR octet_length(r.values_canonical) NOT BETWEEN 27 AND 58671
                OR octet_length(r.sources_canonical) NOT BETWEEN 19 AND 36847
                OR octet_length(r.submission_canonical) NOT BETWEEN 141 AND 4161
                OR r.values_digest<>sha256(r.values_canonical)
                OR r.sources_digest<>sha256(r.sources_canonical)
                OR r.submission_digest<>sha256(r.submission_canonical))
         OR EXISTS(SELECT 1 FROM case_procedural_fact_revisions GROUP BY family,id
            HAVING min(revision)<>1 OR max(revision)<>count(*))",
        &[],
    ).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent());
    }
    let mut family: Option<String> = None;
    let mut id = Uuid::nil();
    let mut revision = 0_i64;
    loop {
        let rows = tx.query(
            "SELECT r.family,r.id,r.revision,r.case_id,f.parent_resolution_id
             FROM case_procedural_fact_revisions r JOIN case_procedural_facts f USING(family,id,case_id)
             WHERE $1::text IS NULL OR (r.family COLLATE \"C\",r.id,r.revision)>($1 COLLATE \"C\",$2,$3)
             ORDER BY r.family COLLATE \"C\",r.id,r.revision LIMIT 64",
            &[&family, &id, &revision],
        ).map_err(port)?;
        for row in &rows {
            let current_family: String = row.try_get(0).map_err(|_| inconsistent())?;
            let current_id: Uuid = row.try_get(1).map_err(|_| inconsistent())?;
            let current_revision: i64 = row.try_get(2).map_err(|_| inconsistent())?;
            let case = CaseId::from_uuid(row.try_get(3).map_err(|_| inconsistent())?);
            let parent: Option<Uuid> = row.try_get(4).map_err(|_| inconsistent())?;
            let target = match (current_family.as_str(), parent) {
                ("resolution", None) => FactTarget::Resolution(ResolutionId::from_uuid(current_id)),
                ("notification", Some(parent)) => FactTarget::Notification {
                    id: NotificationId::from_uuid(current_id),
                    resolution_id: ResolutionId::from_uuid(parent),
                },
                _ => return Err(inconsistent()),
            };
            let selected =
                FactRevision::new(u32::try_from(current_revision).map_err(|_| inconsistent())?)
                    .map_err(|_| inconsistent())?;
            crate::procedural_fact_postgres::storage::detail(
                &mut tx,
                case,
                target,
                Some(selected),
                &crate::RingSha256Hasher,
            )
            .map_err(|_| inconsistent())?;
            family = Some(current_family);
            id = current_id;
            revision = current_revision;
        }
        if rows.len() < 64 {
            return tx.rollback().map_err(port);
        }
    }
}
fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "procedural fact inventory is inconsistent; restore a consistent database".into(),
    )
}
