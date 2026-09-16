use super::{
    fact_receipt_matches, fact_snapshot_receipt_matches, resolve_fact_hearings, FactDetail,
    FactResolutionSourceSnapshot, FactResolutionView, FactResolvedSources, FactSourceSelection,
    FactSourceViews, FactSources, ProceduralFactError, ProceduralFactSnapshot,
    ProceduralFactValues, ResolutionSourceMaterial,
};
use crate::ApplicationError;
use domain::{cases::CaseId, crypto::DocumentHasher};

/// Compact identity and readable fields derived from a verified exact parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactResolutionProjection {
    pub snapshot: FactResolutionSourceSnapshot,
    pub view: FactResolutionView,
}

/// Reconstructs only the parent's immediate historical sources. Captured support
/// admission is checked against its receipt, never repeated against document bytes.
/// Withdrawn parents remain readable; no current head or authorization is inferred.
pub fn resolve_fact_resolution(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    selection: &FactSourceSelection,
    material: Option<&ResolutionSourceMaterial>,
) -> Result<Option<FactResolutionProjection>, ApplicationError> {
    let (reference, material) = match (selection.resolution(), material) {
        (None, None) => return Ok(None),
        (Some(reference), Some(material)) => (reference, material),
        _ => {
            return Err(inconsistent(
                "resolution material presence differs from exact selection",
            ))
        }
    };
    let parent = &material.snapshot;
    if parent.root.case_id() != case_id
        || parent.root.id() != reference.id
        || parent.metadata.revision != reference.revision
    {
        return Err(inconsistent(
            "resolution material scope or exact reference differs",
        ));
    }
    let snapshot = ProceduralFactSnapshot::Resolution(Box::new(parent.clone()));
    fact_snapshot_receipt_matches(hasher, &snapshot)
        .map_err(|_| inconsistent("resolution values or reconstructed receipt are inconsistent"))?;
    let selected = FactSourceSelection::from_values(&ProceduralFactValues::Resolution(Box::new(
        parent.values.clone(),
    )));
    let hearings = resolve_fact_hearings(hasher, case_id, &selected, material.hearing.as_slice())?;
    let direct_supports = match (selected.direct_supports(), &material.admitted_support) {
        ([], None) => vec![],
        ([reference], Some(support))
            if reference.reference() == support.reference
                && reference.digest() == support.digest =>
        {
            vec![support.clone()]
        }
        _ => {
            return Err(inconsistent(
                "historical resolution support differs from declared provenance",
            ))
        }
    };
    let (hearing_sources, hearing_views) = hearings
        .into_iter()
        .map(|projection| (projection.snapshot, projection.view))
        .unzip();
    let detail = FactDetail {
        snapshot,
        sources: FactSources {
            resolved: FactResolvedSources {
                resolution: None,
                participants: vec![],
                hearing_results: hearing_sources,
            },
            views: FactSourceViews {
                resolution: None,
                participants: vec![],
                hearing_results: hearing_views,
            },
            direct_supports,
        },
    };
    fact_receipt_matches(hasher, &detail)
        .map_err(|_| inconsistent("reconstructed resolution sources differ from their receipt"))?;
    Ok(Some(FactResolutionProjection {
        snapshot: FactResolutionSourceSnapshot {
            case_id,
            reference,
            values_digest: parent.metadata.values_digest,
            submission_digest: parent.metadata.receipt.submission_digest,
            status: parent.metadata.status,
        },
        view: FactResolutionView {
            reference,
            class: parent.values.class().clone(),
            issuer: parent.values.issuer().clone(),
            issued_at: parent.values.issued_at(),
            summary: parent.values.summary().clone(),
        },
    }))
}

fn inconsistent(message: &str) -> ApplicationError {
    ProceduralFactError::StoredInconsistent(message.into()).into()
}
