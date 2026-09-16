use super::port;
use application::ApplicationError;
use domain::judicial_calendars::{JudicialCalendarId, JudicialCalendarRevision};
use postgres::Client;
use uuid::Uuid;
pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let broken:bool=tx.query_one("SELECT
        EXISTS(SELECT 1 FROM judicial_calendars c LEFT JOIN judicial_calendar_revisions r ON r.calendar_id=c.id AND r.revision=1 WHERE r.calendar_id IS NULL OR c.initial_revision<>1)
        OR EXISTS(SELECT 1 FROM judicial_calendar_revisions r LEFT JOIN judicial_calendars c ON c.id=r.calendar_id WHERE c.id IS NULL OR r.revision NOT BETWEEN 1 AND 4294967295 OR octet_length(r.values_canonical) NOT BETWEEN 99 AND 191910 OR octet_length(r.submission_canonical) NOT BETWEEN 91 AND 4095 OR r.values_digest<>sha256(r.values_canonical) OR r.submission_digest<>sha256(r.submission_canonical))
        OR EXISTS(SELECT 1 FROM judicial_calendar_revisions r LEFT JOIN users u ON u.id=r.recorded_by WHERE u.id IS NULL)
        OR EXISTS(SELECT 1 FROM judicial_calendar_revisions GROUP BY calendar_id HAVING min(revision)<>1 OR max(revision)<>count(*))",&[]).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent());
    }
    let mut id: Option<Uuid> = None;
    let mut revision = 0_i64;
    loop {
        let rows=tx.query("SELECT calendar_id,revision FROM judicial_calendar_revisions WHERE $1::uuid IS NULL OR (calendar_id,revision)>($1,$2) ORDER BY calendar_id,revision LIMIT 64",&[&id,&revision]).map_err(port)?;
        for row in &rows {
            let selected =
                JudicialCalendarId::from_uuid(row.try_get(0).map_err(|_| inconsistent())?);
            let number: i64 = row.try_get(1).map_err(|_| inconsistent())?;
            let selected_revision =
                JudicialCalendarRevision::new(u32::try_from(number).map_err(|_| inconsistent())?)
                    .map_err(|_| inconsistent())?;
            crate::judicial_calendar_postgres::storage::detail(
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
        "judicial calendar inventory is inconsistent; restore a consistent database".into(),
    )
}
