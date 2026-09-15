use crate::case_stages::{decode, query::CHANGE_SELECT};
use crate::postgres_case_stages_schema::{inconsistent, port};
use crate::RingSha256Hasher;
use application::ApplicationError;
use postgres::Client;

/// Checks historical context and canonical values without loading encrypted content.
pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let broken:bool=client.query_one(
        "WITH history AS (
            SELECT case_id,revision,stage,change_kind,from_stage FROM case_stage_revisions
            UNION ALL SELECT case_id,stage_revision,stage,'initial',NULL FROM case_initial_stage_registrations
        ), ordered AS (
            SELECT *,lag(revision) OVER w previous_revision,lag(stage) OVER w previous_stage
            FROM history WINDOW w AS (PARTITION BY case_id ORDER BY revision)
        ) SELECT EXISTS(SELECT 1 FROM ordered WHERE revision<>COALESCE(previous_revision+1,1)
            OR (previous_revision IS NULL AND (change_kind NOT IN ('initial','adoption') OR from_stage IS NOT NULL))
            OR (previous_revision IS NOT NULL AND (from_stage IS DISTINCT FROM previous_stage
                OR NOT ((previous_stage='investigation' AND change_kind='to_intermediate')
                    OR (previous_stage='intermediate' AND change_kind='to_trial')))))
        OR EXISTS(SELECT 1 FROM case_stage_revisions s
            LEFT JOIN cases c ON c.id=s.case_id
            LEFT JOIN users u ON u.id=s.recorded_by
            LEFT JOIN case_administration_revisions a ON a.case_id=s.case_id AND a.revision=s.administration_revision
            LEFT JOIN documents d ON d.id=s.support_id AND d.version=s.support_version
            LEFT JOIN documents r ON r.id=s.receipt_id AND r.version=s.receipt_version
            WHERE c.id IS NULL OR u.id IS NULL OR a.case_id IS NULL OR a.nuc IS NULL
                OR a.administrative_status IS DISTINCT FROM 'active'
                OR d.case_id IS DISTINCT FROM s.case_id OR d.digest IS DISTINCT FROM s.support_digest
                OR d.name IS DISTINCT FROM s.support_name
                OR (s.receipt_id IS NOT NULL AND (r.case_id IS DISTINCT FROM s.case_id
                    OR r.digest IS DISTINCT FROM s.receipt_digest OR r.name IS DISTINCT FROM s.receipt_name))
                OR s.revision NOT BETWEEN 1 AND 4294967295
                OR s.recorded_at_seconds NOT BETWEEN -62135596800 AND 253402300799
                OR s.recorded_at_nanoseconds NOT BETWEEN 0 AND 999999999
                OR NOT case_administration_text_valid(s.recorded_by_email,254,FALSE)
                OR NOT case_stage_recording_valid(s)
                OR CASE WHEN case_stage_values_canonical(s)
                    THEN s.values_digest<>sha256(case_stage_values_bytes(s)) ELSE TRUE END)",
        &[],
    ).map_err(|_|inconsistent())?.get(0);
    if broken {
        return Err(inconsistent());
    }
    // Decode in bounded pages, retaining exact historical dates and captured actors.
    let mut cursor: Option<(uuid::Uuid, i64)> = None;
    loop {
        let case = cursor.map(|value| value.0);
        let revision = cursor.map(|value| value.1);
        let rows=client.query(&format!("{CHANGE_SELECT} WHERE ($1::uuid IS NULL OR (s.case_id,s.revision)>($1,$2::bigint)) ORDER BY s.case_id,s.revision LIMIT 100"),&[&case,&revision]).map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in &rows {
            decode::changed(row, &RingSha256Hasher).map_err(|_| inconsistent())?;
        }
        let last = rows.last().expect("nonempty stage inventory page");
        cursor = Some((last.get("case_id"), last.get("revision")));
    }
    Ok(())
}
