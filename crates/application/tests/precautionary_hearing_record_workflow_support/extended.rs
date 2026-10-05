use super::*;
use domain::hearings::{HearingNote, HearingVenue};

pub fn values_input(values: &PrecautionaryHearingValues) -> PrecautionaryHearingValuesInput {
    crate::record_review_support::values_input(values)
}

pub fn merge_history(
    into: &mut MeasureDecisionRecordHistoryEvidence,
    added: &MeasureDecisionRecordHistoryEvidence,
) {
    for entry in &added.records.judicial.groups {
        if let Some(old) = into
            .records
            .judicial
            .groups
            .iter()
            .find(|old| old.origin.operation_id == entry.origin.operation_id)
        {
            assert_eq!(old, entry);
        } else {
            into.records.judicial.groups.push(entry.clone());
        }
    }
    for entry in &added.records.administrative {
        if let Some(old) = into
            .records
            .administrative
            .iter()
            .find(|old| old.origin.operation_id == entry.origin.operation_id)
        {
            assert_eq!(old, entry);
        } else {
            into.records.administrative.push(entry.clone());
        }
    }
    for entry in &added.decisions {
        if let Some(old) = into
            .decisions
            .iter()
            .find(|old| old.origin.operation_id == entry.origin.operation_id)
        {
            assert_eq!(old, entry);
        } else {
            into.decisions.push(entry.clone());
        }
    }
}

impl Fixture {
    pub fn replace(previous: &PrecautionaryHearingRecordStoredOperation) -> Self {
        let mut fixture = Self::schedule();
        let prior = &previous.capture;
        let mut input = values_input(&prior.review.resolved_values);
        input.venue = HearingVenue::new("Replacement court").unwrap();
        fixture.material.observed_context = crate::precautionary_receipt_support::later_context();
        fixture.command.operation_id =
            PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(31));
        fixture.command.hearing_id = prior.review.command.hearing_id;
        fixture.command.change = PrecautionaryHearingChange::Replace {
            expected_revision: prior.review.result_revision,
            expected_capture_digest: prior.capture_digest,
            context: crate::precautionary_receipt_support::expectation(
                &fixture.material.observed_context,
            ),
            values: PrecautionaryHearingValues::new(input).unwrap(),
            reason: HearingNote::new("Correct the communicated venue").unwrap(),
        };
        fixture.material.history = Some(previous.history.clone());
        fixture.material.record_history = previous.history.record_history.clone();
        fixture
    }

    pub fn cancel(previous: &PrecautionaryHearingRecordStoredOperation) -> Self {
        let mut fixture = Self::replace(previous);
        fixture.command.operation_id =
            PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(32));
        fixture.command.change = PrecautionaryHearingChange::Cancel {
            expected_revision: previous.capture.review.result_revision,
            expected_capture_digest: previous.capture.capture_digest,
            reason: HearingNote::new("Cancel the communicated appointment").unwrap(),
        };
        fixture.material.selected_sources = None;
        fixture
    }

    pub fn targets(&mut self, targets: Vec<PrecautionaryMeasureRef>) {
        let values = match &mut self.command.change {
            PrecautionaryHearingChange::Schedule { values, .. }
            | PrecautionaryHearingChange::Replace { values, .. } => values,
            _ => unreachable!(),
        };
        let mut input = values_input(values);
        input.review_targets = targets;
        *values = PrecautionaryHearingValues::new(input).unwrap();
    }

    pub fn full_operation(
        &self,
        recorded_at: OffsetDateTime,
    ) -> PrecautionaryHearingRecordStoredOperation {
        let capture = self.checked().into_capture(&Hasher, recorded_at).unwrap();
        let mut history = match &self.material.history {
            Some(history) => history.clone(),
            None => PrecautionaryHearingRecordHistoryEvidence {
                origin: precautionary_hearing_origin_with_decision_history(
                    &Hasher,
                    &capture,
                    &self.material.record_history,
                )
                .unwrap(),
                captures: vec![],
                record_history: self.material.record_history.clone(),
            },
        };
        merge_history(&mut history.record_history, &self.material.record_history);
        history.captures.push(capture.clone());
        precautionary_hearing_history_with_decision_history_matches(
            &Hasher,
            &history.captures,
            &history.origin,
            &history.record_history,
        )
        .unwrap();
        PrecautionaryHearingRecordStoredOperation { capture, history }
    }

    pub fn replay_store(&self, operation: PrecautionaryHearingRecordStoredOperation) -> MockStore {
        let mut store = MockStore::new();
        let actor = self.actor.clone();
        let case_id = self.case_id;
        let command = self.command.clone();
        store
            .expect_prepare()
            .times(1)
            .withf(move |a, c, cmd, _| *a == actor && *c == case_id && *cmd == command)
            .return_once(move |_, _, _, _| {
                Ok(PrecautionaryHearingRecordPreparation::Replay(Box::new(
                    operation,
                )))
            });
        store
    }
}

pub fn submit(fixture: Fixture) -> PrecautionaryHearingRecordStoredOperation {
    let review = fixture.review();
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| prepared.into_operation(now()));
    harness(store, identity(fixture.actor))
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&review),
        )
        .unwrap()
}
