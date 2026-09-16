use super::{inconsistent, port};
use application::hearings::{HearingId, HearingRevision};
use application::ApplicationError;
use domain::cases::CaseId;
use postgres::Client;
use uuid::Uuid;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let client = &mut tx;
    let broken: bool = client
        .query_one(
            "SELECT
        EXISTS(SELECT 1 FROM case_hearings h LEFT JOIN case_hearing_revisions r
            ON r.hearing_id=h.id AND r.revision=1 WHERE r.hearing_id IS NULL)
        OR EXISTS(SELECT 1 FROM case_hearing_revisions r LEFT JOIN case_hearings h
            ON h.id=r.hearing_id AND h.case_id=r.case_id WHERE h.id IS NULL
                OR octet_length(r.values_canonical) NOT BETWEEN 27 AND 10726
                OR octet_length(r.submission_canonical) NOT BETWEEN 108 AND 4120
                OR r.values_digest<>sha256(r.values_canonical)
                OR r.submission_digest<>sha256(r.submission_canonical))
        OR EXISTS(SELECT 1 FROM case_hearing_revisions GROUP BY hearing_id
            HAVING min(revision)<>1 OR max(revision)<>count(*))",
            &[],
        )
        .map_err(port)?
        .get(0);
    if broken {
        return Err(inconsistent());
    }
    let mut id = Uuid::nil();
    let mut revision = 0_i64;
    loop {
        let rows = client
            .query(
                "SELECT hearing_id,revision,case_id
            FROM case_hearing_revisions WHERE (hearing_id,revision)>($1,$2)
            ORDER BY hearing_id,revision LIMIT 64",
                &[&id, &revision],
            )
            .map_err(port)?;
        for row in &rows {
            let case = CaseId::from_uuid(row.try_get(2).map_err(|_| inconsistent())?);
            let hearing = HearingId::from_uuid(row.try_get(0).map_err(|_| inconsistent())?);
            let number: i64 = row.try_get(1).map_err(|_| inconsistent())?;
            let selected_revision =
                HearingRevision::new(u32::try_from(number).map_err(|_| inconsistent())?)
                    .map_err(|_| inconsistent())?;
            crate::hearing_postgres::storage::detail(
                client,
                case,
                hearing,
                Some(selected_revision),
                &crate::RingSha256Hasher,
            )
            .map_err(|_| inconsistent())?;
            id = row.try_get(0).map_err(|_| inconsistent())?;
            revision = row.try_get(1).map_err(|_| inconsistent())?;
        }
        if rows.len() < 64 {
            return tx.rollback().map_err(port);
        }
    }
}
