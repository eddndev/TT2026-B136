use super::*;
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    DomainError,
};

struct ConstantHasher;
impl DocumentHasher for ConstantHasher {
    fn hash_bytes(&self, _: &[u8]) -> Sha256Digest {
        Sha256Digest::from_array([0; 32])
    }
    fn hash_stream(&self, _: &mut dyn std::io::Read) -> Result<Sha256Digest, DomainError> {
        Ok(self.hash_bytes(&[]))
    }
}

#[test]
fn context_scope_and_captured_value_commitments_are_validated_before_disclosure() {
    let actor = reader(Role::Owner);
    let context = context();
    let wrong_case = CaseId::new();
    let store = returning(&actor, wrong_case, context.clone());
    assert!(service(store, identity(&actor), clock())
        .get("session", wrong_case)
        .is_err());

    let mut material = crate::context_support::changed(crate::context_support::trial());
    let constant = Sha256Digest::from_array([0; 32]);
    material.administration.values_digest = constant;
    material.stage_administration.values_digest = constant;
    let stage = crate::context_support::changed_mut(&mut material);
    stage.administration_digest = constant;
    stage.values_digest = constant;
    let inconsistent = PrecautionaryContext::new(&ConstantHasher, material).unwrap();
    let case = inconsistent.material().case_id;
    let store = returning(&actor, case, inconsistent);
    assert!(service(store, identity(&actor), clock())
        .get("session", case)
        .is_err());
}

#[test]
fn unsupported_and_regressing_observation_clocks_prevent_disclosure() {
    let actor = reader(Role::Paralegal);
    let context = context();
    let case = context.material().case_id;
    let non_utc = now().to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap());
    let year_zero = time::Date::from_calendar_date(0, time::Month::January, 1)
        .unwrap()
        .midnight()
        .assume_utc();
    for invalid in [non_utc, year_zero] {
        let clock = Arc::new(ReadClock(Mutex::new(vec![invalid])));
        assert!(service(MockReads::new(), identity(&actor), clock)
            .get("session", case)
            .is_err());
    }
    for final_time in [non_utc, year_zero, now() - time::Duration::nanoseconds(1)] {
        let store = returning(&actor, case, context.clone());
        let clock = Arc::new(ReadClock(Mutex::new(vec![now(), final_time])));
        assert!(service(store, identity(&actor), clock)
            .get("session", case)
            .is_err());
    }
}

#[test]
fn final_observation_covers_both_administration_and_stage_source_times_exactly() {
    let actor = reader(Role::Litigator);
    for administration_seconds in [5, 20] {
        let mut material = crate::context_support::changed(crate::context_support::trial());
        crate::context_support::observed_newer(&mut material);
        material.administration.changed_at = at() + time::Duration::seconds(administration_seconds);
        let context = PrecautionaryContext::new(&Hasher, material).unwrap();
        let case = context.material().case_id;
        let floor = context
            .material()
            .administration
            .changed_at
            .max(context.material().stage.recorded_at());
        for final_time in [floor - time::Duration::nanoseconds(1), floor] {
            let store = returning(&actor, case, context.clone());
            let clock = Arc::new(ReadClock(Mutex::new(vec![at(), final_time])));
            let result = service(store, identity(&actor), clock).get("session", case);
            if final_time == floor {
                assert_eq!(result.unwrap(), context);
            } else {
                assert!(result.is_err());
            }
        }
    }
}
