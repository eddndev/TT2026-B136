mod literals;

pub use literals::*;

use application::precautionary_measures::*;
use domain::precautionary_hearings::{MeasureId, PrecautionaryMeasureRef};
use domain::precautionary_measures::{
    MeasureDecisionId, MeasureDecisionOperationId, MeasureDecisionOutcome,
    MeasureDecisionOutcomeInput, MeasureEffect, MeasureProposal,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::decision_vector_support::{capture, Hasher};

fn id(value: u128) -> MeasureId {
    MeasureId::from_uuid(Uuid::from_u128(value))
}

pub fn later(substitute: bool) -> MeasureDecisionGroupCapture {
    let previous = capture(false);
    let empty = MeasureHistoryEvidence { groups: vec![] };
    let origin = measure_group_origin(&Hasher, &previous, &empty).unwrap();
    let source = &previous.measures[0];
    let reference = PrecautionaryMeasureRef::new(
        source.result.id,
        source.result.revision,
        source.capture_digest,
    );
    let mut command = previous.review.command.clone();
    command.operation_id =
        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(if substitute { 14 } else { 10 }));
    command.decision_id =
        MeasureDecisionId::from_uuid(Uuid::from_u128(if substitute { 15 } else { 11 }));
    command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![if substitute {
            MeasureEffect::Substitute {
                predecessors: vec![reference],
                successors: vec![
                    MeasureProposal {
                        id: id(13),
                        values: source.result.values.clone(),
                    },
                    MeasureProposal {
                        id: id(12),
                        values: source.result.values.clone(),
                    },
                ],
            }
        } else {
            MeasureEffect::Confirm {
                previous: reference,
            }
        }]))
        .unwrap();
    let mut material = previous.review.material.clone();
    material.predecessors = vec![OwnedMeasureMaterial {
        owner: MeasureGroupRef {
            operation_id: previous.review.command.operation_id,
            decision_id: previous.review.command.decision_id,
            group_digest: previous.capture_digest,
        },
        capture: source.clone(),
    }];
    material.result_sources = if substitute {
        [13, 9, 12]
            .into_iter()
            .map(|value| MeasureResultSources {
                id: id(value),
                sources: source.result.sources.clone(),
            })
            .collect()
    } else {
        vec![MeasureResultSources {
            id: id(9),
            sources: source.result.sources.clone(),
        }]
    };
    let actor = previous.review.actor.clone();
    let case_id = previous.review.case_id;
    let evidence = MeasureHistoryEvidence {
        groups: vec![MeasureGroupEvidence {
            origin,
            capture: previous,
        }],
    };
    prepare_measure_decision_with_history(&Hasher, &actor, case_id, command, material, &evidence)
        .unwrap()
        .into_group_capture(
            &Hasher,
            OffsetDateTime::from_unix_timestamp_nanos(if substitute { 11 } else { 10 }).unwrap(),
        )
        .unwrap()
}
