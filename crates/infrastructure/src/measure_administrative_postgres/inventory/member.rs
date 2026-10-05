use super::super::{decode, inconsistent, port};
use application::{measure_corrections::*, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    precautionary_measures::{MeasureSupervision, MeasureValues},
};
use postgres::{Row, Transaction};

const BOUNDS: &str = "octet_length(r.values_canonical) BETWEEN 91 AND 20113
 AND octet_length(r.values_view::text)<=32768 AND octet_length(r.values_digest)=32
 AND octet_length(r.capture_digest)=32 AND octet_length(r.subject_values_digest)=32
 AND octet_length(r.family)<=2 AND octet_length(r.action)<=14 AND octet_length(r.validity)<=16";

pub(super) fn validate(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &MeasureAdministrativeCommand,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let op = command.operation_id.as_uuid();
    let counts = tx.query_one("SELECT
        (SELECT count(*) FROM (SELECT 1 FROM case_measure_revisions WHERE owner_operation=$1 LIMIT 2) members),
        (SELECT count(*) FROM (SELECT 1 FROM case_measures WHERE root_operation=$1 LIMIT 1) roots)",
        &[&op]).map_err(port)?;
    if counts.get::<_, i64>(0) != 1 || counts.get::<_, i64>(1) != 0 {
        return Err(inconsistent(
            "advertised administrative owner has missing or extra members or roots",
        ));
    }
    let previous = tx.query_opt(&format!("SELECT r.*,root.root_operation,root.initial_revision,o.family AS root_family
        FROM case_measure_revisions r JOIN case_measures root ON root.id=r.measure_id AND root.case_id=r.case_id
        JOIN case_measure_operations o ON o.operation_id=root.root_operation AND o.case_id=root.case_id
        WHERE r.case_id=$1 AND r.measure_id=$2 AND r.revision=$3 AND {BOUNDS}
        AND octet_length(o.family)<=2"),
        &[&case.as_uuid(), &command.target.id().as_uuid(), &i64::from(command.target.revision().get())],
    ).map_err(port)?.ok_or_else(|| inconsistent("administrative target declaration is absent or oversized"))?;
    if decode::digest(previous.get("capture_digest"))? != command.target.digest()
        || !matches!(
            previous.get::<_, String>("family").as_str(),
            "m1" | "m2" | "c1"
        )
        || previous.get::<_, String>("validity") != "valid"
        || previous.get::<_, i64>("initial_revision") != 1
        || !matches!(
            previous.get::<_, String>("root_family").as_str(),
            "g1" | "g2"
        )
    {
        return Err(inconsistent(
            "administrative target identity, validity or root differs",
        ));
    }
    let original = values(&previous, hasher)?;
    let (expected, validity) = match &command.action {
        MeasureAdministrativeAction::Correct(correction) => (
            original.correct_record(correction).map_err(inconsistent)?,
            "valid",
        ),
        MeasureAdministrativeAction::MarkEnteredInError => (original, "entered_in_error"),
    };
    let row = tx
        .query_opt(
            &format!(
                "SELECT r.* FROM case_measure_revisions r
        WHERE r.owner_operation=$1 AND {BOUNDS}"
            ),
            &[&op],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("advertised administrative member exceeds bounds"))?;
    let revision = command
        .target
        .revision()
        .next()
        .ok_or_else(|| inconsistent("administrative result revision overflows"))?;
    if row.get::<_, uuid::Uuid>("case_id") != case.as_uuid()
        || row.get::<_, uuid::Uuid>("measure_id") != command.target.id().as_uuid()
        || row.get::<_, i64>("revision") != i64::from(revision.get())
        || row.get::<_, String>("family") != "c1"
        || row.get::<_, String>("validity") != validity
        || row.get::<_, String>("action") != previous.get::<_, String>("action")
        || values(&row, hasher)? != expected
    {
        return Err(inconsistent(
            "advertised administrative member differs from the exact command",
        ));
    }
    Ok(())
}

fn values(row: &Row, hasher: &dyn DocumentHasher) -> Result<MeasureValues, ApplicationError> {
    let bytes: Vec<u8> = row.get("values_canonical");
    let values = crate::measure_decision_codec::measure_values(&bytes, &row.get("values_view"))?;
    let subject = values.subject();
    let supervisor = match values.supervision() {
        MeasureSupervision::Known { participant, .. } => (
            Some(participant.id().as_uuid()),
            Some(i64::from(participant.revision().get())),
        ),
        MeasureSupervision::Unknown { .. } => (None, None),
    };
    if hasher.hash_bytes(&bytes) != decode::digest(row.get("values_digest"))?
        || row.get::<_, uuid::Uuid>("subject_id") != subject.id.as_uuid()
        || row.get::<_, i64>("subject_revision") != i64::from(subject.revision.get())
        || decode::digest(row.get("subject_values_digest"))? != subject.values_digest
        || (
            row.get::<_, Option<uuid::Uuid>>("supervisor_id"),
            row.get::<_, Option<i64>>("supervisor_revision"),
        ) != supervisor
    {
        return Err(inconsistent(
            "advertised measure values or source selectors differ",
        ));
    }
    Ok(values)
}
