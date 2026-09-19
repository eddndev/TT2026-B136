use super::{decode, inconsistent, port};
use application::{cases::CurrentCaseAdministration, ApplicationError};
use domain::{
    case_administration::{CaseAdministrativeStatus, CaseRevision},
    cases::{CaseId, CaseMetadata},
    crypto::DocumentHasher,
};
use postgres::{Row, Transaction};

pub(crate) fn captured(
    tx: &mut Transaction<'_>,
    row: &Row,
    case: CaseId,
    hasher: &dyn DocumentHasher,
) -> Result<CurrentCaseAdministration, ApplicationError> {
    let revision: Option<i64> = row
        .try_get("recorded_administration_revision")
        .map_err(inconsistent)?;
    let digest: Option<Vec<u8>> = row
        .try_get("recorded_administration_digest")
        .map_err(inconsistent)?;
    let title: Option<String> = row
        .try_get("recorded_administration_title")
        .map_err(inconsistent)?;
    let reference: Option<String> = row
        .try_get("recorded_administration_reference")
        .map_err(inconsistent)?;
    match (revision, digest, title, reference) {
        (Some(revision), Some(digest), None, None) => {
            let revision = CaseRevision::new(decode::counter(revision)?).map_err(inconsistent)?;
            let snapshot = crate::hearing_postgres::administration(tx, case, revision, hasher)?;
            if snapshot.values_digest != decode::digest(digest)?
                || snapshot.values.status() != CaseAdministrativeStatus::Active
            {
                return Err(inconsistent("captured fact administration differs"));
            }
            Ok(CurrentCaseAdministration::Recorded(Box::new(snapshot)))
        }
        (None, None, Some(title), Some(reference)) => {
            let baseline=tx.query_opt("SELECT title,reference,required_initial_revision FROM cases WHERE id=$1 AND octet_length(title)<=800 AND octet_length(reference)<=400", &[&case.as_uuid()]).map_err(port)?.ok_or_else(||inconsistent("fact baseline absent or unbounded"))?;
            if baseline
                .get::<_, Option<i64>>("required_initial_revision")
                .is_some()
                || baseline.get::<_, String>("title") != title
                || baseline.get::<_, String>("reference") != reference
            {
                return Err(inconsistent("fact unrevised baseline differs"));
            }
            let metadata = CaseMetadata::new(&title, &reference).map_err(inconsistent)?;
            if metadata.title() != title || metadata.reference() != reference {
                return Err(inconsistent("noncanonical fact baseline"));
            }
            Ok(CurrentCaseAdministration::Unrevised(metadata))
        }
        _ => Err(inconsistent("incomplete fact administration capture")),
    }
}
