use application::{resource_activities::*, ApplicationError};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    resource: ResourceId,
    selection: ResourceActivitySelection,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceActivitySources, ApplicationError> {
    if selection.resource.id != resource {
        return Err(ResourceActivityError::SourceMismatch.into());
    }
    let source = crate::procedural_resource_postgres::storage::detail(
        tx,
        case,
        resource,
        Some(selection.resource.revision),
        hasher,
    )?;
    if source.receipt.capture_digest != selection.resource.capture_digest {
        return Err(ResourceActivityError::SourceMismatch.into());
    }
    let act = selection
        .act
        .map(|reference| {
            let row = crate::procedural_resource_postgres::storage::detail(
                tx,
                case,
                resource,
                Some(reference.resource_revision),
                hasher,
            )?;
            if row.receipt.capture_digest != reference.capture_digest
                || row
                    .act
                    .as_ref()
                    .is_none_or(|act| act.id != reference.id || act.revision != reference.revision)
            {
                return Err(ResourceActivityError::SourceMismatch.into());
            }
            Ok::<_, ApplicationError>(row)
        })
        .transpose()?;
    let target = match selection.target {
        ResourceActivityTarget::Hearing {
            id,
            revision,
            submission_digest,
        } => {
            let row =
                crate::hearing_postgres::storage::detail(tx, case, id, Some(revision), hasher)?;
            if row.snapshot.receipt.submission_digest != submission_digest {
                return Err(ResourceActivityError::SourceMismatch.into());
            }
            ResourceActivityTargetDetail::Hearing(Box::new(row))
        }
        ResourceActivityTarget::Deadline {
            id,
            revision,
            capture_digest,
        } => {
            let row =
                crate::deadline_postgres::storage::detail(tx, case, id, Some(revision), hasher)?;
            if row.receipt.capture_digest != capture_digest {
                return Err(ResourceActivityError::SourceMismatch.into());
            }
            ResourceActivityTargetDetail::Deadline(Box::new(row))
        }
    };
    Ok(ResourceActivitySources {
        resource: source,
        act,
        target,
    })
}
