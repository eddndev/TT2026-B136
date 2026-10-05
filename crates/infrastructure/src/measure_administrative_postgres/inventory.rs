use super::{decode, inconsistent, port, storage};
use application::{identity::Principal, measure_corrections::*, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    identity::{Role, UserId},
    precautionary_measures::MeasureCorrectionOperationId,
};
use postgres::{Row, Transaction};
use uuid::Uuid;

pub(super) mod audit;
mod member;

/// Validate one original declaration at a time without retaining unrelated ancestry.
pub(crate) fn validate(tx: &mut Transaction<'_>) -> Result<(), ApplicationError> {
    let hasher = &crate::RingSha256Hasher;
    let mut after: Option<Uuid> = None;
    loop {
        let rows = tx
            .query(
                "SELECT operation_id,case_id FROM case_measure_administrations
             WHERE ($1::uuid IS NULL OR operation_id>$1) ORDER BY operation_id LIMIT 8",
                &[&after],
            )
            .map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: Uuid = row.get("operation_id");
            let case = CaseId::from_uuid(row.get("case_id"));
            let payload = storage::raw(tx, case, MeasureCorrectionOperationId::from_uuid(id))?;
            let command = advertised(tx, &payload, hasher)?;
            member::validate(tx, case, &command, hasher)?;
            after = Some(id);
        }
    }
    Ok(())
}

pub(super) fn advertised(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureAdministrativeCommand, ApplicationError> {
    if row.get::<_, String>("family") != "a1" {
        return Err(inconsistent(
            "administrative payload has another owner family",
        ));
    }
    let command = decode::command(row, hasher)?;
    let case = CaseId::from_uuid(row.get("case_id"));
    let actor = Principal {
        id: UserId::from_uuid(row.get("recorded_by")),
        email: row.get("recorded_by_email"),
        role: row
            .get::<_, String>("recorded_by_role")
            .parse()
            .map_err(inconsistent)?,
    };
    if !matches!(actor.role, Role::Owner | Role::Litigator) {
        return Err(inconsistent(
            "administrative declaration has an unsupported actor role",
        ));
    }
    let submission = decode::digest(row.get("submission_digest"))?;
    let review = decode::digest(row.get("review_digest"))?;
    let capture = decode::digest(row.get("capture_digest"))?;
    if hasher.hash_bytes(&measure_administrative_submission_bytes(
        &actor, case, &command,
    )?) != submission
        || capture != decode::digest(row.get("owner_digest"))?
    {
        return Err(inconsistent(
            "administrative declaration differs from its original commitments",
        ));
    }
    let revision = command
        .target
        .revision()
        .next()
        .ok_or_else(|| inconsistent("administrative revision overflows"))?;
    let marker =
        format!(
        "ma1:case:{case}:operation:{}:measure:{}:revision:{}:submission:{}:review:{}:capture:{}",
        command.operation_id, command.target.id(), revision.get(),
        submission.to_hex(), review.to_hex(), capture.to_hex(),
    );
    audit::verify(
        tx,
        row.get("audit_sequence"),
        &actor.email,
        "measure_administrative.recorded",
        &marker,
        audit::time(row)?,
        hasher,
    )?;
    Ok(command)
}
