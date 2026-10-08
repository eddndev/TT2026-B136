use super::{inconsistent, sources};
use application::{
    measure_corrections::{MeasureAdministrativeRef, OwnedJudicialMeasure, OwnedMeasureRecord},
    precautionary_measures::*,
    ApplicationError,
};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::MeasureEffect,
};
use postgres::Transaction;

pub(super) fn predecessors(
    _hasher: &dyn DocumentHasher,
    case: CaseId,
    refs: &[PrecautionaryMeasureRef],
    evidence: &MeasureDecisionRecordHistoryEvidence,
) -> Result<Vec<OwnedMeasureRecord>, ApplicationError> {
    let mut result = Vec::with_capacity(refs.len());
    for reference in refs {
        let mut found = None;
        for g in &evidence.records.judicial.groups {
            for capture in &g.capture.measures {
                if capture.case_id == case
                    && capture.result.id == reference.id()
                    && capture.result.revision == reference.revision()
                {
                    if found.is_some() || capture.capture_digest != reference.digest() {
                        return Err(inconsistent("selected mixed predecessor ownership differs"));
                    }
                    found = Some(OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V1(
                        Box::new(OwnedMeasureMaterial {
                            owner: MeasureGroupRef {
                                operation_id: g.origin.operation_id,
                                decision_id: g.origin.decision_id,
                                group_digest: g.origin.group_digest,
                            },
                            capture: capture.clone(),
                        }),
                    )));
                }
            }
        }
        for g in &evidence.decisions {
            for capture in &g.capture.measures {
                if capture.case_id == case
                    && capture.result.id == reference.id()
                    && capture.result.revision == reference.revision()
                {
                    if found.is_some() || capture.capture_digest != reference.digest() {
                        return Err(inconsistent("selected mixed predecessor ownership differs"));
                    }
                    found = Some(OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V2(
                        Box::new(OwnedMeasureMaterialV2 {
                            owner: MeasureGroupRef {
                                operation_id: g.origin.operation_id,
                                decision_id: g.origin.decision_id,
                                group_digest: g.origin.group_digest,
                            },
                            capture: capture.clone(),
                        }),
                    )));
                }
            }
        }
        for a in &evidence.records.administrative {
            for capture in &a.capture.records {
                if capture.case_id == case
                    && capture.result.id == reference.id()
                    && capture.result.revision == reference.revision()
                {
                    if found.is_some() || capture.capture_digest != reference.digest() {
                        return Err(inconsistent("selected mixed predecessor ownership differs"));
                    }
                    found = Some(OwnedMeasureRecord::Administrative {
                        owner: MeasureAdministrativeRef {
                            operation_id: a.origin.operation_id,
                            capture_digest: a.origin.capture_digest,
                        },
                        capture: Box::new(capture.clone()),
                    });
                }
            }
        }
        result.push(found.ok_or_else(|| inconsistent("selected mixed predecessor is absent"))?);
    }
    Ok(result)
}

pub(super) fn result_sources(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &MeasureDecisionCommand,
    predecessors: &[OwnedMeasureRecord],
    hasher: &dyn DocumentHasher,
) -> Result<Vec<MeasureResultSources>, ApplicationError> {
    let mut result = Vec::new();
    for effect in command.outcome.changes().unwrap_or(&[]) {
        match effect {
            MeasureEffect::Impose(proposal) => result.push(MeasureResultSources {
                id: proposal.id,
                sources: sources::selected(tx, case, &proposal.values, hasher)?,
            }),
            MeasureEffect::Modify { previous, values } => result.push(MeasureResultSources {
                id: previous.id(),
                sources: sources::selected(tx, case, values, hasher)?,
            }),
            MeasureEffect::Confirm { previous }
            | MeasureEffect::Revoke { previous }
            | MeasureEffect::Cease { previous } => {
                result.push(retained(case, *previous, predecessors)?)
            }
            MeasureEffect::Substitute {
                predecessors: selected,
                successors,
            } => {
                for reference in selected {
                    result.push(retained(case, *reference, predecessors)?);
                }
                for proposal in successors {
                    result.push(MeasureResultSources {
                        id: proposal.id,
                        sources: sources::selected(tx, case, &proposal.values, hasher)?,
                    });
                }
            }
        }
    }
    result.sort_by_key(|item| item.id.as_uuid());
    Ok(result)
}
fn retained(
    case: CaseId,
    reference: PrecautionaryMeasureRef,
    predecessors: &[OwnedMeasureRecord],
) -> Result<MeasureResultSources, ApplicationError> {
    let mut selected = None;
    for record in predecessors {
        let (record_case, selected_ref, material) = match record {
            OwnedMeasureRecord::Judicial(m) => (m.case_id(), m.reference(), m.sources()),
            OwnedMeasureRecord::Administrative { capture, .. } => (
                capture.case_id,
                PrecautionaryMeasureRef::new(
                    capture.result.id,
                    capture.result.revision,
                    capture.capture_digest,
                ),
                &capture.result.sources,
            ),
        };
        if record_case == case && selected_ref == reference && selected.replace(material).is_some()
        {
            return Err(inconsistent(
                "retained mixed result has duplicate predecessors",
            ));
        }
    }
    Ok(MeasureResultSources {
        id: reference.id(),
        sources: selected
            .ok_or_else(|| inconsistent("retained mixed result lacks its exact predecessor"))?
            .clone(),
    })
}
