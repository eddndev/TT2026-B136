use super::{
    MeasureAdministrativeDependant, MeasureAdministrativeDependencyInventory,
    MeasureAdministrativeRef, MeasureDependencyHearingRef, MeasureDependencyJudicialOwner,
};
use crate::{
    precautionary_hearings::PrecautionaryHearingCapture,
    precautionary_measures::{
        MeasureDecisionAnchorMaterial, MeasureDecisionCommand, MeasureGroupRef,
    },
};
use domain::{
    precautionary_hearings::{PrecautionaryHearingPurpose, PrecautionaryMeasureRef},
    precautionary_measures::MeasureEffect,
};

/// Enumerates direct uses only after the complete supplied forest has been validated.
pub(super) fn collect(
    target: PrecautionaryMeasureRef,
    inventory: &MeasureAdministrativeDependencyInventory,
) -> Vec<MeasureAdministrativeDependant> {
    let mut report = Vec::new();
    for entry in &inventory.records.records.judicial.groups {
        let group = &entry.capture;
        judicial(
            &mut report,
            target,
            MeasureDependencyJudicialOwner::V1(MeasureGroupRef {
                operation_id: group.decision.operation_id,
                decision_id: group.decision.decision_id,
                group_digest: group.capture_digest,
            }),
            &group.review.command,
            group.decision.anchor.as_ref(),
        );
    }
    for entry in &inventory.records.decisions {
        let group = &entry.capture;
        judicial(
            &mut report,
            target,
            MeasureDependencyJudicialOwner::V2(MeasureGroupRef {
                operation_id: group.decision.operation_id,
                decision_id: group.decision.decision_id,
                group_digest: group.capture_digest,
            }),
            &group.review.command,
            group.decision.anchor.as_ref(),
        );
    }
    for entry in &inventory.records.records.administrative {
        if entry.capture.review.command.target == target {
            report.push(MeasureAdministrativeDependant::Administrative {
                owner: MeasureAdministrativeRef {
                    operation_id: entry.capture.review.command.operation_id,
                    capture_digest: entry.capture.capture_digest,
                },
                target,
            });
        }
    }
    for prefix in &inventory.hearings {
        for capture in &prefix.captures {
            if review_uses(capture, target) {
                report.push(MeasureAdministrativeDependant::Review {
                    hearing: hearing_reference(capture),
                    target,
                });
            }
        }
    }
    report.sort_by_cached_key(order_key);
    report.dedup();
    report
}

fn judicial(
    report: &mut Vec<MeasureAdministrativeDependant>,
    target: PrecautionaryMeasureRef,
    owner: MeasureDependencyJudicialOwner,
    command: &MeasureDecisionCommand,
    anchor: Option<&MeasureDecisionAnchorMaterial>,
) {
    let uses_target = command.outcome.changes().is_some_and(|effects| {
        effects.iter().any(|effect| match effect {
            MeasureEffect::Impose(_) => false,
            MeasureEffect::Confirm { previous }
            | MeasureEffect::Modify { previous, .. }
            | MeasureEffect::Revoke { previous }
            | MeasureEffect::Cease { previous } => *previous == target,
            MeasureEffect::Substitute { predecessors, .. } => predecessors.contains(&target),
        })
    });
    if uses_target {
        report.push(MeasureAdministrativeDependant::Judicial {
            owner: owner.clone(),
            target,
        });
    }
    if let Some(MeasureDecisionAnchorMaterial::Precautionary(capture)) = anchor {
        if review_uses(capture, target) {
            report.push(MeasureAdministrativeDependant::ReviewAnchor {
                owner,
                hearing: hearing_reference(capture),
                target,
            });
        }
    }
}

fn review_uses(capture: &PrecautionaryHearingCapture, target: PrecautionaryMeasureRef) -> bool {
    capture.review.resolved_values.purpose() == PrecautionaryHearingPurpose::Review
        && capture
            .review
            .resolved_values
            .review_targets()
            .contains(&target)
}

fn hearing_reference(capture: &PrecautionaryHearingCapture) -> MeasureDependencyHearingRef {
    MeasureDependencyHearingRef {
        hearing_id: capture.review.command.hearing_id,
        revision: capture.review.result_revision,
        operation_id: capture.review.command.operation_id,
        capture_digest: capture.capture_digest,
    }
}

fn order_key(dependant: &MeasureAdministrativeDependant) -> Vec<u8> {
    let mut key = Vec::new();
    let target = match dependant {
        MeasureAdministrativeDependant::Judicial { owner, target } => {
            key.push(0);
            judicial_key(&mut key, owner);
            target
        }
        MeasureAdministrativeDependant::Administrative { owner, target } => {
            key.push(1);
            key.extend_from_slice(owner.operation_id.as_uuid().as_bytes());
            key.extend_from_slice(owner.capture_digest.as_bytes());
            target
        }
        MeasureAdministrativeDependant::Review { hearing, target } => {
            key.push(2);
            hearing_key(&mut key, hearing);
            target
        }
        MeasureAdministrativeDependant::ReviewAnchor {
            owner,
            hearing,
            target,
        } => {
            key.push(3);
            judicial_key(&mut key, owner);
            hearing_key(&mut key, hearing);
            target
        }
    };
    key.extend_from_slice(target.id().as_uuid().as_bytes());
    key.extend_from_slice(&target.revision().get().to_be_bytes());
    key.extend_from_slice(target.digest().as_bytes());
    key
}

fn judicial_key(key: &mut Vec<u8>, owner: &MeasureDependencyJudicialOwner) {
    let reference = match owner {
        MeasureDependencyJudicialOwner::V1(reference) => {
            key.push(0);
            reference
        }
        MeasureDependencyJudicialOwner::V2(reference) => {
            key.push(1);
            reference
        }
    };
    key.extend_from_slice(reference.operation_id.as_uuid().as_bytes());
    key.extend_from_slice(reference.decision_id.as_uuid().as_bytes());
    key.extend_from_slice(reference.group_digest.as_bytes());
}

fn hearing_key(key: &mut Vec<u8>, hearing: &MeasureDependencyHearingRef) {
    key.extend_from_slice(hearing.hearing_id.as_uuid().as_bytes());
    key.extend_from_slice(&hearing.revision.get().to_be_bytes());
    key.extend_from_slice(hearing.operation_id.as_uuid().as_bytes());
    key.extend_from_slice(hearing.capture_digest.as_bytes());
}
