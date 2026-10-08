use super::*;
use domain::hearings::HearingNote;

impl Fixture {
    pub fn multiple() -> Self {
        Self::from_pure(
            crate::measure_decision_fixtures::Fixture::multiple(),
            empty_history(),
        )
    }

    pub fn confirm(previous: &MeasureDecisionStoredOperation) -> Self {
        let sequence = previous
            .group
            .review
            .command
            .operation_id
            .as_uuid()
            .as_u128()
            - 99;
        let next = crate::effect_support::LaterFixture::next(
            &previous.group,
            &previous.measure_history,
            sequence,
        );
        Self::from_pure(next.request, next.evidence)
    }

    pub fn modify(previous: &MeasureDecisionStoredOperation) -> Self {
        let mut fixture = Self::confirm(previous);
        let prior = &previous.group.measures[0];
        let mut input = crate::effect_support::values_input(&prior.result.values);
        input.conditions = HearingNote::new("Conditions modified by the new decision").unwrap();
        fixture.effects(vec![MeasureEffect::Modify {
            previous: crate::measure_decision_fixtures::reference(prior),
            values: MeasureValues::new(input),
        }]);
        fixture
    }

    pub fn revoke(previous: &MeasureDecisionStoredOperation) -> Self {
        let mut fixture = Self::confirm(previous);
        fixture.effects(vec![MeasureEffect::Revoke {
            previous: crate::measure_decision_fixtures::reference(&previous.group.measures[0]),
        }]);
        fixture
    }

    pub fn cease(previous: &MeasureDecisionStoredOperation) -> Self {
        let mut fixture = Self::confirm(previous);
        fixture.effects(vec![MeasureEffect::Cease {
            previous: crate::measure_decision_fixtures::reference(&previous.group.measures[0]),
        }]);
        fixture
    }

    pub fn substitute(previous: &MeasureDecisionStoredOperation) -> Self {
        let mut next = crate::effect_support::substitution(&previous.group, &[90, 100]);
        next.evidence =
            crate::effect_support::append_history(&previous.measure_history, &previous.group);
        Self::from_pure(next.request, next.evidence)
    }

    pub fn effects(&mut self, changes: Vec<MeasureEffect>) {
        self.command.outcome =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(changes)).unwrap();
    }

    pub fn initial_anchor() -> Self {
        let mut fixture = crate::measure_decision_fixtures::Fixture::single();
        crate::decision_anchor_support::attach_initial(
            &mut fixture,
            crate::decision_anchor_support::ordinary_initial(),
        );
        Self::from_pure(fixture, empty_history())
    }

    pub fn precautionary_anchor() -> Self {
        let mut fixture = crate::measure_decision_fixtures::Fixture::single();
        let hearing = crate::precautionary_receipt_support::Fixture::schedule().capture(None, at());
        crate::decision_anchor_support::attach_precautionary(&mut fixture, hearing);
        Self::from_pure(fixture, empty_history())
    }

    pub fn review_anchor_no_change(previous: &MeasureDecisionStoredOperation) -> Self {
        let evidence =
            crate::effect_support::append_history(&previous.measure_history, &previous.group);
        let targets: Vec<_> = previous
            .group
            .measures
            .iter()
            .map(crate::measure_decision_fixtures::reference)
            .collect();
        let hearing = crate::decision_anchor_support::review_hearing(
            900,
            &targets,
            &evidence,
            at() + Duration::seconds(1),
        );
        let mut fixture = crate::decision_anchor_support::fresh_no_change(900);
        crate::decision_anchor_support::attach_precautionary(&mut fixture, hearing);
        Self::from_pure(fixture, evidence)
    }
}
