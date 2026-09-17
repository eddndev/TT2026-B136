use super::{inconsistent, DeadlineInputMaterial, DeadlineSourceDetail};
use crate::{
    hearing_results::hearing_result_receipt_matches,
    procedural_facts::{fact_receipt_matches, ProceduralFactSnapshot},
    ApplicationError,
};
use domain::{
    crypto::DocumentHasher,
    deadline_triggers::{
        FactTriggerDigests, HearingTriggerDigests, TriggerMaterial, TriggerSelection,
    },
    procedural_facts::FactDeclaration,
};

pub(super) fn checked_source<'a>(
    hasher: &dyn DocumentHasher,
    selection: &TriggerSelection,
    material: &'a DeadlineInputMaterial,
) -> Result<Option<TriggerMaterial<'a>>, ApplicationError> {
    let (exact, head) = match (&selection.source, &material.source, &material.source_head) {
        (FactDeclaration::Unknown(_), None, None) => return Ok(None),
        (FactDeclaration::Known(_), Some(exact), Some(head)) => (exact, head),
        _ => {
            return Err(inconsistent(
                "source and head presence differ from selection",
            ))
        }
    };
    receipt(hasher, exact)?;
    receipt(hasher, head)?;
    let (same_identity, order) = match (exact, head) {
        (DeadlineSourceDetail::Fact(a), DeadlineSourceDetail::Fact(b)) => (
            a.snapshot.case_id() == b.snapshot.case_id()
                && a.snapshot.target() == b.snapshot.target(),
            b.snapshot
                .metadata()
                .revision
                .cmp(&a.snapshot.metadata().revision),
        ),
        (DeadlineSourceDetail::HearingResult(a), DeadlineSourceDetail::HearingResult(b)) => (
            a.snapshot.case_id == b.snapshot.case_id
                && a.snapshot.hearing_id == b.snapshot.hearing_id
                && a.snapshot.id == b.snapshot.id,
            b.snapshot.revision.cmp(&a.snapshot.revision),
        ),
        _ => return Err(inconsistent("source and head belong to different families")),
    };
    if !same_identity || order.is_lt() || (order.is_eq() && exact != head) {
        return Err(inconsistent(
            "source head identity, revision or exact contents disagree",
        ));
    }
    Ok(Some(as_trigger(exact)))
}
fn receipt(
    hasher: &dyn DocumentHasher,
    source: &DeadlineSourceDetail,
) -> Result<(), ApplicationError> {
    match source {
        DeadlineSourceDetail::Fact(detail) => fact_receipt_matches(hasher, detail),
        DeadlineSourceDetail::HearingResult(detail) => {
            hearing_result_receipt_matches(hasher, detail)
        }
    }
    .map_err(inconsistent)
}
fn as_trigger(source: &DeadlineSourceDetail) -> TriggerMaterial<'_> {
    match source {
        DeadlineSourceDetail::Fact(detail) => {
            let metadata = detail.snapshot.metadata();
            let digests = FactTriggerDigests {
                values: metadata.values_digest,
                sources: metadata.receipt.sources_digest,
                submission: metadata.receipt.submission_digest,
            };
            match &detail.snapshot {
                ProceduralFactSnapshot::Resolution(snapshot) => TriggerMaterial::Resolution {
                    root: snapshot.root,
                    revision: metadata.revision,
                    values: &snapshot.values,
                    digests,
                },
                ProceduralFactSnapshot::Notification(snapshot) => TriggerMaterial::Notification {
                    root: snapshot.root,
                    revision: metadata.revision,
                    values: &snapshot.values,
                    digests,
                },
            }
        }
        DeadlineSourceDetail::HearingResult(detail) => {
            let snapshot = &detail.snapshot;
            TriggerMaterial::HearingResult {
                case_id: snapshot.case_id,
                hearing_id: snapshot.hearing_id,
                result_id: snapshot.id,
                revision: snapshot.revision,
                values: &snapshot.values,
                digests: HearingTriggerDigests {
                    values: snapshot.values_digest,
                    submission: snapshot.receipt.submission_digest,
                },
            }
        }
    }
}
