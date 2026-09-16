use super::{submission::Submission, *};
use crate::ApplicationError;
use domain::{cases::CaseId, crypto::DocumentHasher};

pub(super) fn inconsistent(message: &str) -> ApplicationError {
    ProceduralFactError::StoredInconsistent(message.into()).into()
}
impl From<&ProceduralFactSnapshot> for FactHistoryEntry {
    fn from(value: &ProceduralFactSnapshot) -> Self {
        Self {
            case_id: value.case_id(),
            target: value.target(),
            metadata: value.metadata().clone(),
        }
    }
}
impl ProceduralFactSnapshot {
    pub fn values(&self) -> ProceduralFactValues {
        match self {
            Self::Resolution(v) => ProceduralFactValues::Resolution(Box::new(v.values.clone())),
            Self::Notification(v) => ProceduralFactValues::Notification(Box::new(v.values.clone())),
        }
    }
}
/// Checks the self-contained receipt; the store also resolves persisted exact sources.
pub fn fact_history_receipt_matches(
    hasher: &dyn DocumentHasher,
    entry: &FactHistoryEntry,
) -> Result<(), ApplicationError> {
    let metadata = &entry.metadata;
    let receipt = &metadata.receipt;
    let valid = match receipt.action {
        FactAction::Record => receipt.expected_revision == 0 && metadata.reason.is_none(),
        FactAction::Correct | FactAction::Withdraw => {
            receipt.expected_revision > 0 && metadata.reason.is_some()
        }
    };
    if !valid
        || metadata.status != receipt.action.resulting_status()
        || receipt.expected_revision.checked_add(1) != Some(metadata.revision.get())
    {
        return Err(inconsistent(
            "receipt action, reason, status or revision differs",
        ));
    }
    super::administration::validate_snapshot(
        hasher,
        entry.case_id,
        &metadata.recorded_administration,
    )?;
    let bytes = Submission {
        actor: metadata.recorded_by.id,
        case_id: entry.case_id,
        operation_id: receipt.operation_id,
        target: entry.target,
        action: receipt.action,
        expected_revision: receipt.expected_revision,
        values_digest: metadata.values_digest,
        sources_digest: receipt.sources_digest,
        reason: metadata.reason.as_ref(),
    }
    .bytes();
    if hasher.hash_bytes(&bytes) != receipt.submission_digest {
        return Err(inconsistent(
            "receipt digest differs from reconstructed submission",
        ));
    }
    Ok(())
}
pub fn fact_snapshot_receipt_matches(
    hasher: &dyn DocumentHasher,
    snapshot: &ProceduralFactSnapshot,
) -> Result<(), ApplicationError> {
    fact_history_receipt_matches(hasher, &FactHistoryEntry::from(snapshot))?;
    if let ProceduralFactSnapshot::Notification(value) = snapshot {
        if value.root.resolution_id() != value.values.resolution().id {
            return Err(inconsistent(
                "notification values change its immutable parent",
            ));
        }
    }
    if fact_values_digest(hasher, &snapshot.values()) != snapshot.metadata().values_digest {
        return Err(inconsistent("value digest differs from declared fact"));
    }
    Ok(())
}
pub fn fact_receipt_matches(
    hasher: &dyn DocumentHasher,
    detail: &FactDetail,
) -> Result<(), ApplicationError> {
    fact_snapshot_receipt_matches(hasher, &detail.snapshot)?;
    validate_source_selection(
        detail.snapshot.case_id(),
        &FactSourceSelection::from_values(&detail.snapshot.values()),
        &detail.sources,
    )?;
    if fact_sources_digest(hasher, &detail.sources)?
        != detail.snapshot.metadata().receipt.sources_digest
    {
        return Err(inconsistent(
            "source digest differs from historical projections",
        ));
    }
    Ok(())
}
pub(super) fn validate_source_selection(
    case_id: CaseId,
    selection: &FactSourceSelection,
    sources: &FactSources,
) -> Result<(), ApplicationError> {
    let resolved = &sources.resolved;
    if resolved.resolution.map(|source| source.reference) != selection.resolution()
        || resolved
            .resolution
            .is_some_and(|source| source.case_id != case_id)
        || resolved.participants.len() != selection.participants().len()
        || resolved.hearing_results.len() != selection.hearing_results().len()
        || sources.direct_supports.len() != selection.direct_supports().len()
    {
        return Err(inconsistent(
            "source scope or exact selection count differs",
        ));
    }
    if resolved
        .participants
        .iter()
        .zip(selection.participants())
        .any(|(source, reference)| source.case_id != case_id || source.reference != *reference)
        || resolved
            .hearing_results
            .iter()
            .zip(selection.hearing_results())
            .any(|(source, reference)| source.case_id != case_id || source.reference != *reference)
        || sources
            .direct_supports
            .iter()
            .zip(selection.direct_supports())
            .any(|(source, reference)| {
                source.reference != reference.reference() || source.digest != reference.digest()
            })
    {
        return Err(inconsistent(
            "source projection differs from declared exact reference",
        ));
    }
    super::source_shape::validate(sources)
}
