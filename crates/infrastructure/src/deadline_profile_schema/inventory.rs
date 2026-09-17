use super::port;
use application::{
    deadline_profiles::{DeadlineProfileId, DeadlineProfileRevision},
    ApplicationError,
};
use postgres::Client;
use uuid::Uuid;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let broken:bool=tx.query_one("SELECT
        EXISTS(SELECT 1 FROM deadline_profiles p LEFT JOIN deadline_profile_revisions r ON r.profile_id=p.id AND r.revision=1 WHERE r.profile_id IS NULL OR p.initial_revision<>1)
        OR EXISTS(SELECT 1 FROM deadline_profiles p LEFT JOIN cases c ON c.id=p.case_id WHERE p.case_id IS NOT NULL AND c.id IS NULL)
        OR EXISTS(SELECT 1 FROM deadline_profile_revisions r LEFT JOIN deadline_profiles p ON p.id=r.profile_id WHERE p.id IS NULL OR r.revision NOT BETWEEN 1 AND 4294967295 OR r.algorithm<>1 OR octet_length(r.definition_canonical) NOT BETWEEN 202 AND 3261697 OR octet_length(r.submission_canonical) NOT BETWEEN 92 AND 4096 OR r.definition_digest<>sha256(r.definition_canonical) OR r.submission_digest<>sha256(r.submission_canonical))
        OR EXISTS(SELECT 1 FROM deadline_profile_revisions r LEFT JOIN users u ON u.id=r.recorded_by WHERE u.id IS NULL)
        OR EXISTS(SELECT 1 FROM deadline_profile_revisions GROUP BY profile_id HAVING min(revision)<>1 OR max(revision)<>count(*))",&[]).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent());
    }
    let mut id: Option<Uuid> = None;
    let mut revision = 0_i64;
    loop {
        let rows=tx.query("SELECT profile_id,revision FROM deadline_profile_revisions WHERE $1::uuid IS NULL OR (profile_id,revision)>($1,$2) ORDER BY profile_id,revision LIMIT 64",&[&id,&revision]).map_err(port)?;
        for row in &rows {
            let selected =
                DeadlineProfileId::from_uuid(row.try_get(0).map_err(|_| inconsistent())?);
            let number: i64 = row.try_get(1).map_err(|_| inconsistent())?;
            let selected_revision =
                DeadlineProfileRevision::new(u32::try_from(number).map_err(|_| inconsistent())?)
                    .map_err(|_| inconsistent())?;
            crate::deadline_profile_postgres::storage::detail(
                &mut tx,
                selected,
                Some(selected_revision),
                &crate::RingSha256Hasher,
            )
            .map_err(|_| inconsistent())?;
            id = Some(selected.as_uuid());
            revision = number;
        }
        if rows.len() < 64 {
            return tx.rollback().map_err(port);
        }
    }
}
fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "deadline profile inventory is inconsistent; restore a consistent database".into(),
    )
}
