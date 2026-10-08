use super::inconsistent;
use application::ApplicationError;

pub(super) struct Budget {
    owners: usize,
    members: usize,
}
impl Budget {
    pub(super) fn new(reserved_owners: usize, reserved_members: usize) -> Self {
        Self {
            owners: reserved_owners,
            members: reserved_members,
        }
    }
    pub(super) fn admit(&mut self, count: usize) -> Result<(), ApplicationError> {
        let members = self
            .members
            .checked_add(count)
            .ok_or_else(|| inconsistent("measure member count overflows"))?;
        if self.owners >= 256 || members > 8192 || count > 32 {
            return Err(inconsistent(
                "measure history exceeds its owner or member budget",
            ));
        }
        self.owners += 1;
        self.members = members;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Budget;

    #[test]
    fn candidate_owner_allows_255_ancestors_and_rejects_the_next() {
        let mut budget = Budget::new(1, 1);
        for _ in 0..255 {
            budget.admit(1).unwrap();
        }
        assert!(budget.admit(1).is_err());
    }

    #[test]
    fn historical_capture_allows_all_256_owners() {
        let mut budget = Budget::new(0, 0);
        for _ in 0..256 {
            budget.admit(32).unwrap();
        }
        assert!(budget.admit(0).is_err());
    }

    #[test]
    fn candidate_and_ancestors_can_exactly_fill_both_budgets() {
        let mut budget = Budget::new(1, 32);
        for _ in 0..255 {
            budget.admit(32).unwrap();
        }
        assert!(budget.admit(1).is_err());
    }

    #[test]
    fn reserved_members_are_counted_independently_of_owner_capacity() {
        let mut budget = Budget::new(0, 32);
        for _ in 0..255 {
            budget.admit(32).unwrap();
        }
        assert!(budget.admit(1).is_err());
        budget.admit(0).unwrap();
    }

    #[test]
    fn no_measure_change_candidate_still_reserves_one_owner() {
        let mut budget = Budget::new(1, 0);
        for _ in 0..255 {
            budget.admit(0).unwrap();
        }
        assert!(budget.admit(0).is_err());
    }
}
