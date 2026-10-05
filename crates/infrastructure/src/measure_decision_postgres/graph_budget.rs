use super::inconsistent;
use application::ApplicationError;
use std::collections::BTreeSet;
use uuid::Uuid;

pub(super) struct HearingBudget {
    rows: BTreeSet<(Uuid, u32)>,
    captures: usize,
    targets: usize,
}

impl HearingBudget {
    pub(super) fn new(reserved_captures: usize, reserved_targets: usize) -> Self {
        Self {
            rows: BTreeSet::new(),
            captures: reserved_captures,
            targets: reserved_targets,
        }
    }

    pub(super) fn admit(
        &mut self,
        id: Uuid,
        revision: u32,
        target_count: usize,
    ) -> Result<bool, ApplicationError> {
        if target_count > 32 {
            return Err(inconsistent("hearing review target count exceeds 32"));
        }
        if self.rows.contains(&(id, revision)) {
            return Ok(false);
        }
        let captures = self
            .captures
            .checked_add(1)
            .ok_or_else(|| inconsistent("hearing capture count overflows"))?;
        let targets = self
            .targets
            .checked_add(target_count)
            .ok_or_else(|| inconsistent("hearing target count overflows"))?;
        if captures > 256 || targets > 8192 {
            return Err(inconsistent(
                "hearing history exceeds its capture or target budget",
            ));
        }
        self.rows.insert((id, revision));
        self.captures = captures;
        self.targets = targets;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::HearingBudget;
    use crate::measure_decision_postgres::history_budget::Budget;
    use uuid::Uuid;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    #[test]
    fn hearing_and_judicial_histories_have_independent_full_capacities() {
        let mut hearings = HearingBudget::new(0, 0);
        let mut groups = Budget::new(0, 0);
        for revision in 1..=256 {
            groups.admit(32).unwrap();
            assert!(hearings.admit(id(1), revision, 32).unwrap());
        }
        assert!(groups.admit(0).is_err());
        assert!(hearings.admit(id(1), 257, 0).is_err());
    }

    #[test]
    fn repeated_exact_rows_are_not_charged_even_after_capacity_is_full() {
        let mut budget = HearingBudget::new(0, 0);
        for revision in 1..=256 {
            assert!(budget.admit(id(1), revision, 32).unwrap());
            assert!(!budget.admit(id(1), revision, 32).unwrap());
        }
        assert!(!budget.admit(id(1), 1, 32).unwrap());
        assert!(budget.admit(id(2), 1, 0).is_err());
    }

    #[test]
    fn identity_and_revision_both_distinguish_captured_rows() {
        let mut budget = HearingBudget::new(253, 0);
        assert!(budget.admit(id(1), 1, 1).unwrap());
        assert!(budget.admit(id(1), 2, 1).unwrap());
        assert!(budget.admit(id(2), 1, 1).unwrap());
        assert!(!budget.admit(id(1), 2, 1).unwrap());
        assert!(budget.admit(id(2), 2, 0).is_err());
    }

    #[test]
    fn a_fresh_candidate_reserves_one_capture_and_its_target_occurrences() {
        let mut budget = HearingBudget::new(1, 32);
        for revision in 1..=255 {
            assert!(budget.admit(id(1), revision, 32).unwrap());
        }
        assert!(!budget.admit(id(1), 255, 32).unwrap());
        assert!(budget.admit(id(1), 256, 0).is_err());
    }

    #[test]
    fn target_occurrences_are_counted_across_distinct_captures() {
        let mut budget = HearingBudget::new(0, 8160);
        assert!(budget.admit(id(1), 1, 32).unwrap());
        assert!(!budget.admit(id(1), 1, 32).unwrap());
        assert!(budget.admit(id(1), 2, 1).is_err());
        assert!(budget.admit(id(1), 2, 0).unwrap());
    }

    #[test]
    fn an_oversized_value_does_not_consume_a_row_or_target_capacity() {
        let mut budget = HearingBudget::new(255, 8160);
        assert!(budget.admit(id(1), 1, 33).is_err());
        assert!(budget.admit(id(1), 1, 32).unwrap());
        assert!(!budget.admit(id(1), 1, 32).unwrap());
    }

    #[test]
    fn unsupported_reservations_fail_without_arithmetic_overflow() {
        for (captures, targets) in [(257, 0), (0, 8193), (usize::MAX, 0), (0, usize::MAX)] {
            let mut budget = HearingBudget::new(captures, targets);
            assert!(budget.admit(id(1), 1, 0).is_err());
        }
    }
}
