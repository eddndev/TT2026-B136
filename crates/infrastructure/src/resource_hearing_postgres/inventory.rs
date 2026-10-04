use super::{inconsistent, port, replay, storage};
use application::ApplicationError;
use domain::{cases::CaseId, resource_hearings::ResourceHearingId};
use postgres::Client;

pub(crate) fn validate_inventory(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let invalid:bool=tx.query_one("SELECT EXISTS(
        SELECT 1 FROM case_resource_hearings h LEFT JOIN case_resource_hearing_revisions r
        ON r.hearing_id=h.id AND r.case_id=h.case_id AND r.resource_id=h.resource_id AND r.revision=h.initial_revision
        WHERE r.hearing_id IS NULL OR h.initial_revision<>1)
        OR EXISTS(SELECT 1 FROM case_resource_hearing_revisions r LEFT JOIN case_resource_hearings h
        ON h.id=r.hearing_id AND h.case_id=r.case_id AND h.resource_id=r.resource_id
        WHERE h.id IS NULL OR r.revision<>1 OR r.submission_digest<>sha256(r.submission_canonical)
        OR r.capture_digest<>sha256(r.capture_canonical))", &[]).map_err(port)?.get(0);
    if invalid {
        return Err(inconsistent(
            "resource hearing roots, captures or hashes differ",
        ));
    }
    let mut after: Option<uuid::Uuid> = None;
    loop {
        let rows=tx.query("SELECT id,case_id FROM case_resource_hearings WHERE ($1::uuid IS NULL OR id>$1) ORDER BY id LIMIT 64",&[&after]).map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: uuid::Uuid = row.get("id");
            let detail = storage::detail(
                &mut tx,
                CaseId::from_uuid(row.get("case_id")),
                ResourceHearingId::from_uuid(id),
                None,
                &crate::RingSha256Hasher,
            )?;
            replay::creation(&mut tx, detail, &crate::RingSha256Hasher)?;
            after = Some(id);
        }
    }
    validate_origins(&mut tx)?;
    tx.commit().map_err(port)
}

fn validate_origins(tx: &mut postgres::Transaction<'_>) -> Result<(), ApplicationError> {
    let mut after: Option<i64> = None;
    loop {
        let rows=tx.query("SELECT sequence,CASE WHEN octet_length(resource)<=512 THEN resource ELSE NULL END AS resource FROM audit_events
            WHERE action='resource_hearing.registered' AND ($1::bigint IS NULL OR sequence>$1) ORDER BY sequence LIMIT 64",&[&after]).map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let marker: Option<String> = row.get("resource");
            let marker =
                marker.ok_or_else(|| inconsistent("resource hearing origin exceeds bounds"))?;
            let fields: Vec<&str> = marker.split(':').collect();
            if fields.len() != 15 || fields[0] != "rhl1" {
                return Err(inconsistent("resource hearing origin framing differs"));
            }
            let case = CaseId::from_uuid(uuid::Uuid::parse_str(fields[2]).map_err(inconsistent)?);
            let id = ResourceHearingId::from_uuid(
                uuid::Uuid::parse_str(fields[6]).map_err(inconsistent)?,
            );
            let detail = storage::detail(tx, case, id, None, &crate::RingSha256Hasher)?;
            let result = replay::creation(tx, detail, &crate::RingSha256Hasher)?;
            if replay::marker(&result) != marker {
                return Err(inconsistent("resource hearing origin differs from capture"));
            }
            after = Some(row.get("sequence"));
        }
    }
    Ok(())
}
