use crate::support::*;
use application::{precautionary_measures::MeasureDecisionError, ApplicationError};
use domain::{
    crypto::Sha256Digest, precautionary_measures::*,
    procedural_time::DeclaredProceduralTime as Declared, typed_participants::CaseSubjectId,
};
use infrastructure::measure_decision_codec::*;
use serde_json::{json, Value};
use time::{Month, UtcOffset};
use uuid::Uuid;

type Decoder = fn(&[u8], &Value) -> Result<(), ApplicationError>;
struct Stored {
    bytes: Vec<u8>,
    view: Value,
    decode: Decoder,
}
impl Stored {
    fn rejects(&self, mutate: impl FnOnce(&mut Value)) {
        let mut view = self.view.clone();
        mutate(&mut view);
        self.rejects_bytes(&self.bytes, &view);
    }
    fn rejects_bytes(&self, bytes: &[u8], view: &Value) {
        assert!(matches!(
            (self.decode)(bytes, view),
            Err(ApplicationError::MeasureDecision(
                MeasureDecisionError::StoredInconsistent(_)
            ))
        ));
    }
    fn exact_keys(&self, paths: &[&str]) {
        for path in paths {
            let keys: Vec<_> = self
                .view
                .pointer(path)
                .unwrap()
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect();
            for key in keys {
                self.rejects(|view| {
                    view.pointer_mut(path)
                        .unwrap()
                        .as_object_mut()
                        .unwrap()
                        .remove(&key);
                });
            }
            self.rejects(|view| {
                view.pointer_mut(path)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .insert("extra".into(), json!(0));
            });
        }
    }
}
fn stored_decision(value: MeasureDecisionValues) -> Stored {
    Stored {
        bytes: value.canonical_bytes(),
        view: decision_view(&value),
        decode: |b, v| decision_values(b, v).map(|_| ()),
    }
}
fn stored_measure(value: MeasureValues) -> Stored {
    Stored {
        bytes: value.canonical_bytes(),
        view: measure_view(&value),
        decode: |b, v| measure_values(b, v).map(|_| ()),
    }
}
fn stored_outcome(value: MeasureDecisionOutcome) -> Stored {
    Stored {
        bytes: value.canonical_bytes(),
        view: outcome_view(&value),
        decode: |b, v| outcome(b, v).map(|_| ()),
    }
}
fn mixed() -> MeasureDecisionOutcome {
    MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Impose(proposal(1)),
        MeasureEffect::Confirm {
            previous: reference(2),
        },
        MeasureEffect::Modify {
            previous: reference(3),
            values: measure(),
        },
        MeasureEffect::Revoke {
            previous: reference(4),
        },
        MeasureEffect::Cease {
            previous: reference(5),
        },
        MeasureEffect::Substitute {
            predecessors: vec![reference(6), reference(7)],
            successors: vec![proposal(8), proposal(9)],
        },
    ]))
    .unwrap()
}
fn unknown_decision() -> MeasureDecisionValues {
    let mut input = decision_input();
    input.declared_at = time();
    MeasureDecisionValues::new(input)
}
fn unknown_measure() -> MeasureValues {
    let mut input = measure_input();
    input.validity = MeasureValidity::new(time(), note("Declared validity"), None).unwrap();
    input.supervision = MeasureSupervision::Unknown {
        reason: note("Not stated"),
    };
    MeasureValues::new(input)
}
fn known_measure() -> MeasureValues {
    let mut input = measure_input();
    input.validity = MeasureValidity::new(
        known_time(
            Declared::second(date(2025, Month::January, 2), 3, 4, 5, Some(UtcOffset::UTC)).unwrap(),
        ),
        note("Declared validity"),
        Some(known_time(
            Declared::second(date(2025, Month::January, 3), 3, 4, 5, Some(UtcOffset::UTC)).unwrap(),
        )),
    )
    .unwrap();
    input.supervision = MeasureSupervision::Known {
        participant: participant(21, 2),
        statement: note("Declared supervisor"),
    };
    MeasureValues::new(input)
}

#[path = "invalid/values.rs"]
mod values;

#[path = "invalid/temporal.rs"]
mod temporal;

#[path = "invalid/effects.rs"]
mod effects;

#[path = "invalid/frames.rs"]
mod frames;
