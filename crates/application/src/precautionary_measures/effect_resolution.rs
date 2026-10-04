use super::{decision_wire::invalid, *};
use crate::ApplicationError;
use domain::{
    precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef},
    precautionary_measures::{MeasureDecisionOutcome, MeasureEffect, MeasureValues},
};

pub(super) fn selections(outcome: &MeasureDecisionOutcome) -> Vec<PrecautionaryMeasureRef> {
    let mut selected = Vec::new();
    for effect in outcome.changes().unwrap_or(&[]) {
        match effect {
            MeasureEffect::Impose(_) => {}
            MeasureEffect::Confirm { previous }
            | MeasureEffect::Modify { previous, .. }
            | MeasureEffect::Revoke { previous }
            | MeasureEffect::Cease { previous } => selected.push(*previous),
            MeasureEffect::Substitute { predecessors, .. } => {
                selected.extend_from_slice(predecessors)
            }
        }
    }
    selected.sort_by_key(|item| item.id().as_uuid());
    selected
}

pub(super) struct EffectResult<'a> {
    pub id: MeasureId,
    pub effect_key: MeasureId,
    pub action: MeasureCaptureAction,
    pub values: &'a MeasureValues,
    pub prior: Option<&'a MeasureCapture>,
}

pub(super) fn effects<'a>(
    command: &'a MeasureDecisionCommand,
    material: &'a MeasureDecisionMaterial,
) -> Result<Vec<EffectResult<'a>>, ApplicationError> {
    let selected = selections(&command.outcome);
    if selected.len() != material.predecessors.len() {
        return Err(invalid("predecessor inventory differs"));
    }
    for (reference, prior) in selected.iter().zip(&material.predecessors) {
        if *reference != capture_reference(&prior.capture) {
            return Err(invalid("predecessor selection differs"));
        }
        if matches!(
            prior.capture.result.action,
            MeasureCaptureAction::Revoke
                | MeasureCaptureAction::Cease
                | MeasureCaptureAction::SubstituteOut
        ) {
            return Err(invalid("terminal declaration cannot acquire effects"));
        }
    }
    let previous =
        |reference: PrecautionaryMeasureRef| -> Result<&'a MeasureCapture, ApplicationError> {
            material
                .predecessors
                .iter()
                .find(|item| capture_reference(&item.capture) == reference)
                .map(|item| &item.capture)
                .ok_or_else(|| invalid("missing exact predecessor"))
        };
    let mut results = Vec::new();
    for effect in command.outcome.changes().unwrap_or(&[]) {
        match effect {
            MeasureEffect::Impose(proposal) => results.push(EffectResult {
                id: proposal.id,
                effect_key: proposal.id,
                action: MeasureCaptureAction::Impose,
                values: &proposal.values,
                prior: None,
            }),
            MeasureEffect::Confirm {
                previous: reference,
            }
            | MeasureEffect::Revoke {
                previous: reference,
            }
            | MeasureEffect::Cease {
                previous: reference,
            }
            | MeasureEffect::Modify {
                previous: reference,
                ..
            } => {
                let prior = previous(*reference)?;
                let (action, values) = match effect {
                    MeasureEffect::Confirm { .. } => {
                        (MeasureCaptureAction::Confirm, &prior.result.values)
                    }
                    MeasureEffect::Revoke { .. } => {
                        (MeasureCaptureAction::Revoke, &prior.result.values)
                    }
                    MeasureEffect::Cease { .. } => {
                        (MeasureCaptureAction::Cease, &prior.result.values)
                    }
                    MeasureEffect::Modify { values, .. } => {
                        if values.subject() != prior.result.values.subject()
                            || values.kind() != prior.result.values.kind()
                        {
                            return Err(invalid(
                                "modification cannot change exact subject or class",
                            ));
                        }
                        (MeasureCaptureAction::Modify, values)
                    }
                    _ => unreachable!(),
                };
                results.push(EffectResult {
                    id: reference.id(),
                    effect_key: reference.id(),
                    action,
                    values,
                    prior: Some(prior),
                });
            }
            MeasureEffect::Substitute {
                predecessors,
                successors,
            } => {
                let subject = previous(predecessors[0])?.result.values.subject().id;
                let key = predecessors
                    .iter()
                    .map(|item| item.id())
                    .chain(successors.iter().map(|item| item.id))
                    .min_by_key(|id| id.as_uuid())
                    .ok_or_else(|| invalid("empty substitution"))?;
                for reference in predecessors {
                    let prior = previous(*reference)?;
                    if prior.result.values.subject().id != subject {
                        return Err(invalid("substitution crosses subjects"));
                    }
                    results.push(EffectResult {
                        id: reference.id(),
                        effect_key: key,
                        action: MeasureCaptureAction::SubstituteOut,
                        values: &prior.result.values,
                        prior: Some(prior),
                    });
                }
                for proposal in successors {
                    if proposal.values.subject().id != subject {
                        return Err(invalid("substitution crosses subjects"));
                    }
                    results.push(EffectResult {
                        id: proposal.id,
                        effect_key: key,
                        action: MeasureCaptureAction::SubstituteIn,
                        values: &proposal.values,
                        prior: None,
                    });
                }
            }
        }
    }
    results.sort_by_key(|item| item.id.as_uuid());
    Ok(results)
}

pub(super) fn capture_reference(capture: &MeasureCapture) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        capture.result.id,
        capture.result.revision,
        capture.capture_digest,
    )
}

pub(super) fn revision(
    prior: Option<&MeasureCapture>,
) -> Result<MeasureRevision, ApplicationError> {
    match prior {
        Some(prior) => prior
            .result
            .revision
            .next()
            .ok_or_else(|| invalid("measure revision overflow")),
        None => Ok(MeasureRevision::initial()),
    }
}

pub(super) fn substitutions(
    command: &MeasureDecisionCommand,
    measures: &[MeasureCapture],
) -> Result<Vec<MeasureSubstitutionCapture>, ApplicationError> {
    let lookup = |id| {
        measures
            .iter()
            .find(|item| item.result.id == id)
            .map(capture_reference)
            .ok_or_else(|| invalid("substitution member missing"))
    };
    let mut relationships = Vec::new();
    for effect in command.outcome.changes().unwrap_or(&[]) {
        if let MeasureEffect::Substitute {
            predecessors,
            successors,
        } = effect
        {
            let key = predecessors
                .iter()
                .map(|item| item.id())
                .chain(successors.iter().map(|item| item.id))
                .min_by_key(|id| id.as_uuid())
                .ok_or_else(|| invalid("empty substitution"))?;
            relationships.push(MeasureSubstitutionCapture {
                effect_key: key,
                predecessors: predecessors
                    .iter()
                    .map(|previous| {
                        Ok(MeasureSubstitutionPredecessor {
                            previous: *previous,
                            result: lookup(previous.id())?,
                        })
                    })
                    .collect::<Result<_, ApplicationError>>()?,
                successors: successors
                    .iter()
                    .map(|item| lookup(item.id))
                    .collect::<Result<_, _>>()?,
            });
        }
    }
    Ok(relationships)
}
