use super::port;
use application::{
    deadlines::{DeadlineId, DeadlineRevision},
    ApplicationError,
};
use domain::cases::CaseId;
use postgres::Client;
use uuid::Uuid;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let broken: bool = tx.query_one("SELECT
        EXISTS(SELECT 1 FROM case_deadlines d LEFT JOIN cases c ON c.id=d.case_id
            LEFT JOIN case_deadline_revisions r ON r.deadline_id=d.id AND r.revision=1
            WHERE c.id IS NULL OR r.deadline_id IS NULL OR d.initial_revision<>1 OR d.case_id<>r.case_id)
        OR EXISTS(SELECT 1 FROM case_deadline_revisions r LEFT JOIN case_deadlines d ON d.id=r.deadline_id
            LEFT JOIN users u ON u.id=r.recorded_by LEFT JOIN users responsible ON responsible.id=r.responsible_id
            LEFT JOIN deadline_profile_revisions p ON p.profile_id=r.profile_id AND p.revision=r.profile_revision
            LEFT JOIN deadline_profiles root ON root.id=r.profile_id
            WHERE d.id IS NULL OR d.case_id<>r.case_id OR responsible.id IS NULL OR p.profile_id IS NULL
                OR (r.recorded_by IS NOT NULL AND (u.id IS NULL OR r.recorded_by_email IS NULL))
                OR (r.recorded_by IS NULL AND (r.recorded_by_email IS NOT NULL
                    OR substring(r.submission_canonical FROM 1 FOR 5) IS DISTINCT FROM convert_to('DLTX2','UTF8')
                    OR r.action IS DISTINCT FROM 'reevaluate'))
                OR (root.case_id IS NOT NULL AND root.case_id<>r.case_id)
                OR r.revision NOT BETWEEN 1 AND 4294967295 OR (r.revision=1)<>(r.action='register')
                OR r.review_digest<>sha256(r.review_canonical) OR r.capture_digest<>sha256(r.capture_canonical)
                OR r.submission_digest<>sha256(r.submission_canonical)
                OR r.observed_administration_digest<>sha256(r.observed_administration_canonical))
        OR EXISTS(SELECT 1 FROM case_deadline_revisions GROUP BY deadline_id HAVING min(revision)<>1 OR max(revision)<>count(*))", &[]).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent());
    }
    // Null user columns only admit technical candidates above. Every receipt,
    // exact dependency and adjacent transition is verified by the Rust reader.
    let mut id: Option<Uuid> = None;
    let mut revision = 0_i64;
    loop {
        let rows = tx
            .query(
                "SELECT deadline_id,case_id,revision FROM case_deadline_revisions
            WHERE $1::uuid IS NULL OR (deadline_id,revision)>($1,$2)
            ORDER BY deadline_id,revision LIMIT 64",
                &[&id, &revision],
            )
            .map_err(port)?;
        for row in &rows {
            let selected = DeadlineId::from_uuid(row.try_get(0).map_err(|_| inconsistent())?);
            let case = CaseId::from_uuid(row.try_get(1).map_err(|_| inconsistent())?);
            let number: i64 = row.try_get(2).map_err(|_| inconsistent())?;
            let selected_revision =
                DeadlineRevision::new(u32::try_from(number).map_err(|_| inconsistent())?)
                    .map_err(|_| inconsistent())?;
            crate::deadline_postgres::storage::detail(
                &mut tx,
                case,
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
        "deadline inventory is inconsistent; restore a consistent database".into(),
    )
}
