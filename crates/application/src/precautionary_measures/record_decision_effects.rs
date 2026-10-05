use super::{decision_wire::invalid, record_decision_wire::capture_reference, *};
use crate::{
    measure_corrections::{MeasureCaptureValidity, RecordView},
    ApplicationError,
};
use domain::{
    precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef},
    precautionary_measures::{MeasureEffect, MeasureValues},
};

pub(super) struct RecordEffect<'a> {
    pub id: MeasureId,
    pub effect_key: MeasureId,
    pub action: MeasureCaptureAction,
    pub values: &'a MeasureValues,
    pub prior: Option<RecordView<'a>>,
}

pub(super) fn effects<'a>(
    command: &'a MeasureDecisionCommand,
    predecessors: &[RecordView<'a>],
) -> Result<Vec<RecordEffect<'a>>, ApplicationError> {
    let selected = super::effect_resolution::selections(&command.outcome);
    if selected.len() != predecessors.len() {
        return Err(invalid("predecessor inventory differs"));
    }
    for (reference, prior) in selected.iter().zip(predecessors) {
        if *reference != prior.reference() {
            return Err(invalid("predecessor selection differs"));
        }
        if prior.validity() != MeasureCaptureValidity::Valid {
            return Err(invalid("entered-in-error record cannot acquire effects"));
        }
        if matches!(
            prior.last_action(),
            MeasureCaptureAction::Revoke
                | MeasureCaptureAction::Cease
                | MeasureCaptureAction::SubstituteOut
        ) {
            return Err(invalid("terminal declaration cannot acquire effects"));
        }
    }
    let previous = |reference: PrecautionaryMeasureRef| {
        predecessors
            .iter()
            .copied()
            .find(|prior| prior.reference() == reference)
            .ok_or_else(|| invalid("missing exact predecessor"))
    };
    let mut results = Vec::new();
    for effect in command.outcome.changes().unwrap_or(&[]) {
        match effect {
            MeasureEffect::Impose(proposal) => results.push(RecordEffect {
                id: proposal.id,
                effect_key: proposal.id,
                action: MeasureCaptureAction::Impose,
                values: &proposal.values,
                prior: None,
            }),
            MeasureEffect::Confirm {
                previous: reference,
            }
            | MeasureEffect::Modify {
                previous: reference,
                ..
            }
            | MeasureEffect::Revoke {
                previous: reference,
            }
            | MeasureEffect::Cease {
                previous: reference,
            } => {
                let prior = previous(*reference)?;
                let (action, values) = match effect {
                    MeasureEffect::Confirm { .. } => {
                        (MeasureCaptureAction::Confirm, prior.values())
                    }
                    MeasureEffect::Revoke { .. } => (MeasureCaptureAction::Revoke, prior.values()),
                    MeasureEffect::Cease { .. } => (MeasureCaptureAction::Cease, prior.values()),
                    MeasureEffect::Modify { values, .. } => {
                        if values.subject() != prior.values().subject()
                            || values.kind() != prior.values().kind()
                        {
                            return Err(invalid(
                                "modification cannot change exact subject or class",
                            ));
                        }
                        (MeasureCaptureAction::Modify, values)
                    }
                    _ => unreachable!(),
                };
                results.push(RecordEffect {
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
                let subject = previous(predecessors[0])?.values().subject().id;
                let key = predecessors
                    .iter()
                    .map(|item| item.id())
                    .chain(successors.iter().map(|item| item.id))
                    .min_by_key(|id| id.as_uuid())
                    .ok_or_else(|| invalid("empty substitution"))?;
                for reference in predecessors {
                    let prior = previous(*reference)?;
                    if prior.values().subject().id != subject {
                        return Err(invalid("substitution crosses subjects"));
                    }
                    results.push(RecordEffect {
                        id: reference.id(),
                        effect_key: key,
                        action: MeasureCaptureAction::SubstituteOut,
                        values: prior.values(),
                        prior: Some(prior),
                    });
                }
                for proposal in successors {
                    if proposal.values.subject().id != subject {
                        return Err(invalid("substitution crosses subjects"));
                    }
                    results.push(RecordEffect {
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
    results.sort_by_key(|result| result.id.as_uuid());
    Ok(results)
}

pub(super) fn revision(prior: Option<RecordView<'_>>) -> Result<MeasureRevision, ApplicationError> {
    match prior {
        Some(prior) => prior
            .reference()
            .revision()
            .next()
            .ok_or_else(|| invalid("measure revision overflow")),
        None => Ok(MeasureRevision::initial()),
    }
}

pub(super) fn substitutions(
    command: &MeasureDecisionCommand,
    measures: &[MeasureCaptureV2],
) -> Result<Vec<MeasureSubstitutionCapture>, ApplicationError> {
    let lookup = |id| {
        measures
            .iter()
            .find(|member| member.result.id == id)
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
                    .map(|proposal| lookup(proposal.id))
                    .collect::<Result<_, _>>()?,
            });
        }
    }
    relationships.sort_by_key(|relation| relation.effect_key.as_uuid());
    Ok(relationships)
}
