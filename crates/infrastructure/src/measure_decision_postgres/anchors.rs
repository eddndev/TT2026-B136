use super::{decode, inconsistent};
use application::{precautionary_measures::*, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    hearings::{HearingId, HearingRevision},
};
use postgres::{Row, Transaction};
use uuid::Uuid;

pub(super) struct Columns {
    pub kind: &'static str,
    pub hearing_id: Option<Uuid>,
    pub revision: Option<i64>,
    pub values_digest: Option<Vec<u8>>,
    pub submission_digest: Option<Vec<u8>>,
}

pub(super) fn columns(
    reference: &Option<MeasureDecisionAnchorRef>,
) -> Result<Columns, ApplicationError> {
    match reference {
        None => Ok(Columns {
            kind: "none",
            hearing_id: None,
            revision: None,
            values_digest: None,
            submission_digest: None,
        }),
        Some(MeasureDecisionAnchorRef::Initial {
            hearing_id,
            revision,
            values_digest,
            submission_digest,
        }) => Ok(Columns {
            kind: "initial",
            hearing_id: Some(hearing_id.as_uuid()),
            revision: Some(i64::from(revision.get())),
            values_digest: Some(values_digest.as_bytes().to_vec()),
            submission_digest: Some(submission_digest.as_bytes().to_vec()),
        }),
        Some(MeasureDecisionAnchorRef::Precautionary { .. }) => {
            Err(ApplicationError::InvalidInput(
                "durable precautionary hearing anchors are not available".into(),
            ))
        }
    }
}

pub(super) fn reference(row: &Row) -> Result<Option<MeasureDecisionAnchorRef>, ApplicationError> {
    let kind: String = row.get("anchor_kind");
    let id: Option<Uuid> = row.get("anchor_hearing_id");
    let revision: Option<i64> = row.get("anchor_revision");
    let values: Option<Vec<u8>> = row.get("anchor_values_digest");
    let submission: Option<Vec<u8>> = row.get("anchor_submission_digest");
    match (kind.as_str(), id, revision, values, submission) {
        ("none", None, None, None, None) => Ok(None),
        ("initial", Some(id), Some(revision), Some(values), Some(submission)) => {
            Ok(Some(MeasureDecisionAnchorRef::Initial {
                hearing_id: HearingId::from_uuid(id),
                revision: HearingRevision::new(u32::try_from(revision).map_err(inconsistent)?)
                    .map_err(inconsistent)?,
                values_digest: decode::digest(values)?,
                submission_digest: decode::digest(submission)?,
            }))
        }
        _ => Err(inconsistent(
            "stored hearing anchor selectors have an invalid shape",
        )),
    }
}

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    reference: &Option<MeasureDecisionAnchorRef>,
    hasher: &dyn DocumentHasher,
) -> Result<Option<MeasureDecisionAnchorMaterial>, ApplicationError> {
    match reference {
        None => Ok(None),
        Some(MeasureDecisionAnchorRef::Initial {
            hearing_id,
            revision,
            values_digest,
            submission_digest,
        }) => crate::hearing_postgres::load_initial_hearing_anchor(
            tx,
            case,
            *hearing_id,
            *revision,
            *values_digest,
            *submission_digest,
            hasher,
        )
        .map(|detail| Some(MeasureDecisionAnchorMaterial::Initial(Box::new(detail))))
        .map_err(inconsistent),
        Some(MeasureDecisionAnchorRef::Precautionary { .. }) => {
            Err(ApplicationError::InvalidInput(
                "durable precautionary hearing anchors are not available".into(),
            ))
        }
    }
}
