use super::{header, inconsistent, port, stored};
use application::{cases::CurrentCaseAdministration, ApplicationError};
use domain::{
    case_administration::CaseRevision,
    cases::{CaseId, CaseMetadata},
    crypto::DocumentHasher,
};
use postgres::{Row, Transaction};

pub(super) fn captured(
    tx: &mut Transaction<'_>,
    row: &Row,
    case: CaseId,
    hasher: &dyn DocumentHasher,
) -> Result<CurrentCaseAdministration, ApplicationError> {
    let revision = row
        .try_get::<_, Option<i64>>("observed_administration_revision")
        .map_err(inconsistent)?
        .map(|revision| CaseRevision::new(header::counter(revision)?).map_err(inconsistent))
        .transpose()?;
    let captured = at_revision(tx, case, revision, hasher)?;
    let bytes: Vec<u8> = row
        .try_get("observed_administration_canonical")
        .map_err(inconsistent)?;
    let digest = header::digest(
        row.try_get("observed_administration_digest")
            .map_err(inconsistent)?,
    )?;
    if captured.values().canonical_bytes() != bytes || hasher.hash_bytes(&bytes) != digest {
        return Err(inconsistent(
            "deadline administration canonical capture differs",
        ));
    }
    Ok(captured)
}

pub(crate) fn at_revision(
    tx: &mut Transaction<'_>,
    case: CaseId,
    revision: Option<CaseRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<CurrentCaseAdministration, ApplicationError> {
    match revision {
        Some(revision) => Ok(CurrentCaseAdministration::Recorded(Box::new(
            crate::hearing_postgres::administration(tx, case, revision, hasher).map_err(stored)?,
        ))),
        None => {
            let baseline = tx
                .query_opt(
                    "SELECT title,reference,required_initial_revision FROM cases WHERE id=$1
                 AND octet_length(title)<=800 AND octet_length(reference)<=400",
                    &[&case.as_uuid()],
                )
                .map_err(port)?
                .ok_or_else(|| {
                    inconsistent("deadline original administration absent or unbounded")
                })?;
            if baseline
                .try_get::<_, Option<i64>>("required_initial_revision")
                .map_err(inconsistent)?
                .is_some()
            {
                return Err(inconsistent(
                    "deadline invented an unrevised administration",
                ));
            }
            let title: String = baseline.try_get("title").map_err(inconsistent)?;
            let reference: String = baseline.try_get("reference").map_err(inconsistent)?;
            let metadata = CaseMetadata::new(&title, &reference).map_err(inconsistent)?;
            if metadata.title() != title || metadata.reference() != reference {
                return Err(inconsistent(
                    "noncanonical original deadline administration",
                ));
            }
            Ok(CurrentCaseAdministration::Unrevised(metadata))
        }
    }
}
