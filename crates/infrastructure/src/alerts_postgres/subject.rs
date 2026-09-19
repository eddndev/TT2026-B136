use super::{codec, stored};
use application::{
    alerts::*,
    deadlines::{DeadlineAttention, DeadlineRevision, DeadlineStatus},
    hearings::{HearingRevision, HearingStatus},
    ApplicationError,
};
use domain::{crypto::DocumentHasher, identity::UserId};
use postgres::Transaction;
use time::OffsetDateTime;

pub(super) struct Verified {
    pub subject: AlertSubject,
    pub origin: AlertOrigin,
    pub activity_at: Option<OffsetDateTime>,
    pub attention_pending: bool,
    pub review: bool,
    pub ended: Option<AlertResolutionReason>,
    pub responsible: Option<UserId>,
    pub subject_title: String,
    pub case_title: String,
    pub case_reference: String,
}
pub(super) fn load(
    tx: &mut Transaction<'_>,
    subject: AlertSubject,
    hasher: &dyn DocumentHasher,
    now: OffsetDateTime,
) -> Result<Verified, ApplicationError> {
    crate::cases::storage::detail(tx, subject.case_id(), hasher)?;
    let mut result = Verified {
        subject,
        origin: AlertOrigin {
            revision: 1,
            evidence_digest: hasher.hash_bytes(&[]),
        },
        activity_at: None,
        attention_pending: false,
        review: false,
        ended: None,
        responsible: None,
        subject_title: String::new(),
        case_title: String::new(),
        case_reference: String::new(),
    };
    match subject {
        AlertSubject::Hearing { case_id, id } => {
            let detail = crate::hearing_postgres::storage::detail(tx, case_id, id, None, hasher)?;
            let snapshot = detail.snapshot;
            let metadata = hearing_metadata(tx, &snapshot, hasher)?;
            result.case_title = metadata.title().into();
            result.case_reference = metadata.reference().into();
            result.origin = AlertOrigin {
                revision: snapshot.revision.get(),
                evidence_digest: snapshot.receipt.submission_digest,
            };
            result.subject_title = format!("Audiencia {}", snapshot.values.kind().as_str());
            if snapshot.status == HearingStatus::Scheduled {
                result.activity_at = Some(snapshot.values.scheduled_at().utc());
            } else {
                result.ended = Some(AlertResolutionReason::CancelledHearing);
            }
        }
        AlertSubject::Deadline { case_id, id } => {
            let detail = crate::deadline_postgres::storage::detail(tx, case_id, id, None, hasher)?;
            let current =
                crate::deadline_postgres::current_in_transaction(tx, &detail, hasher, now)?;
            result.origin = AlertOrigin {
                revision: detail.revision.get(),
                evidence_digest: detail.receipt.capture_digest,
            };
            result.subject_title = detail.definition.title.as_str().into();
            let administration = detail.calculation.material.administration.values();
            result.case_title = administration.metadata().title().into();
            result.case_reference = administration.metadata().reference().into();
            result.responsible = Some(detail.definition.responsible);
            result.activity_at = current.operational().due_at();
            result.review = current.operational().requires_review();
            result.attention_pending = matches!(detail.attention, DeadlineAttention::Pending);
            if detail.status == DeadlineStatus::Retired {
                result.ended = Some(AlertResolutionReason::TargetRetired);
            }
        }
    }
    Ok(result)
}
pub(super) fn verify_origin(
    tx: &mut Transaction<'_>,
    record: &AlertRecord,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let (digest, title, metadata) = match record.subject {
        AlertSubject::Hearing { case_id, id } => {
            let snapshot = crate::hearing_postgres::storage::detail(
                tx,
                case_id,
                id,
                Some(HearingRevision::new(record.origin.revision).map_err(stored)?),
                hasher,
            )?
            .snapshot;
            let metadata = hearing_metadata(tx, &snapshot, hasher)?;
            (
                snapshot.receipt.submission_digest,
                format!("Audiencia {}", snapshot.values.kind().as_str()),
                metadata,
            )
        }
        AlertSubject::Deadline { case_id, id } => {
            let detail = crate::deadline_postgres::storage::detail(
                tx,
                case_id,
                id,
                Some(DeadlineRevision::new(record.origin.revision).map_err(stored)?),
                hasher,
            )?;
            (
                detail.receipt.capture_digest,
                detail.definition.title.as_str().to_owned(),
                detail
                    .calculation
                    .material
                    .administration
                    .values()
                    .metadata()
                    .clone(),
            )
        }
    };
    if digest != record.origin.evidence_digest
        || title != record.subject_title
        || metadata.title() != record.case_title
        || metadata.reference() != record.case_reference
    {
        return Err(stored("alert context differs from captured source"));
    }
    Ok(())
}
fn hearing_metadata(
    tx: &mut Transaction<'_>,
    snapshot: &application::hearings::HearingSnapshot,
    hasher: &dyn DocumentHasher,
) -> Result<domain::cases::CaseMetadata, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT * FROM case_administration_revisions WHERE case_id=$1 AND revision=$2",
            &[
                &snapshot.case_id.as_uuid(),
                &i64::from(snapshot.recorded_administration_revision.get()),
            ],
        )
        .map_err(super::port)?
        .ok_or_else(|| stored("alert captured case administration is missing"))?;
    let administration = crate::cases::values::decode(&row, hasher)?;
    if administration.values_digest != snapshot.recorded_administration_digest {
        return Err(stored("alert captured case administration differs"));
    }
    Ok(administration.values.metadata().clone())
}

pub(super) fn reason(
    kind: AlertKind,
    current: &Verified,
    now: OffsetDateTime,
) -> Option<AlertResolutionReason> {
    if let Some(reason) = current.ended {
        return Some(reason);
    }
    match kind {
        AlertKind::Upcoming { activity_at, .. } if current.activity_at != Some(activity_at) => {
            Some(AlertResolutionReason::Superseded)
        }
        AlertKind::Upcoming { activity_at, .. } if now >= activity_at => {
            Some(AlertResolutionReason::NoLongerEligible)
        }
        AlertKind::OverdueUnattended { .. } if !current.attention_pending => {
            Some(AlertResolutionReason::AttentionRecorded)
        }
        AlertKind::OverdueUnattended { due_at }
            if current.activity_at != Some(due_at) || now < due_at =>
        {
            Some(AlertResolutionReason::Superseded)
        }
        AlertKind::ReviewRequired if !current.review => {
            Some(AlertResolutionReason::NoLongerEligible)
        }
        AlertKind::DueChangedSoon { current_due_at, .. }
            if current.activity_at != Some(current_due_at) =>
        {
            Some(AlertResolutionReason::Superseded)
        }
        _ => None,
    }
}
pub(super) fn snapshot(current: &Verified) -> serde_json::Value {
    serde_json::json!({"origin":codec::origin(current.origin),"due":codec::optional(current.activity_at),"review":current.review,
        "attention_pending":current.attention_pending,"responsible":current.responsible.map(|id|id.as_uuid())})
}
