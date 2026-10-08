use super::*;
use domain::{
    clock::Clock,
    identity::{Role, UserId},
};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use time::UtcOffset;

fn changed(actor: &Principal, field: u8) -> Principal {
    let mut result = actor.clone();
    match field {
        0 => result.id = UserId::from_uuid(Uuid::from_u128(999)),
        1 => result.email = "changed-current-profile@example.test".into(),
        _ => result.role = Role::Owner,
    }
    result
}

#[test]
fn full_current_principal_changes_block_review_disclosure_and_precommit() {
    for submit in [false, true] {
        for field in 0..3 {
            let fixture = Fixture::single();
            let expected = confirmation(&fixture.review());
            let actor = fixture.actor.clone();
            let other = changed(&actor, field);
            let mut identity = crate::case_support::MockIdentity::new();
            let mut sequence = mockall::Sequence::new();
            identity
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| Ok(actor));
            identity
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| Ok(other));
            let harness = harness(fixture.store(), identity);
            let result = if submit {
                harness
                    .service
                    .submit("session", fixture.case_id, fixture.command, expected)
                    .map(|_| ())
            } else {
                harness
                    .service
                    .prepare("session", fixture.case_id, fixture.command)
                    .map(|_| ())
            };
            assert!(matches!(result, Err(ApplicationError::InvalidSession)));
        }
    }
}

#[test]
fn full_current_principal_changes_after_commit_block_receipt_disclosure() {
    for field in 0..3 {
        let fixture = Fixture::single();
        let expected = confirmation(&fixture.review());
        let committed = Arc::new(AtomicBool::new(false));
        let did_commit = committed.clone();
        let mut store = fixture.store();
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, prepared| {
                let result = prepared.into_operation(now())?;
                did_commit.store(true, Ordering::SeqCst);
                Ok(result)
            });
        let actor = fixture.actor.clone();
        let other = changed(&actor, field);
        let state = committed.clone();
        let mut identity = crate::case_support::MockIdentity::new();
        identity.expect_authenticate().returning(move |_| {
            Ok(if state.load(Ordering::SeqCst) {
                other.clone()
            } else {
                actor.clone()
            })
        });
        let harness = harness(store, identity);
        assert!(matches!(
            harness
                .service
                .submit("session", fixture.case_id, fixture.command, expected),
            Err(ApplicationError::InvalidSession)
        ));
        assert!(committed.load(Ordering::SeqCst));
    }
}

struct SequenceClock {
    values: Vec<OffsetDateTime>,
    position: AtomicUsize,
}
impl Clock for SequenceClock {
    fn now(&self) -> OffsetDateTime {
        let index = self.position.fetch_add(1, Ordering::SeqCst);
        self.values[index.min(self.values.len() - 1)]
    }
}
fn clock(values: Vec<OffsetDateTime>) -> Arc<dyn Clock + Send + Sync> {
    Arc::new(SequenceClock {
        values,
        position: AtomicUsize::new(0),
    })
}

#[test]
fn regressing_or_non_utc_service_observations_cannot_reach_commit() {
    for later in [
        now() - Duration::nanoseconds(1),
        now().to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
    ] {
        let fixture = Fixture::single();
        let expected = confirmation(&fixture.review());
        let harness = harness_with(
            fixture.store(),
            identity(fixture.actor),
            Validator::default(),
            clock(vec![now(), later]),
        );
        assert!(harness
            .service
            .submit("session", fixture.case_id, fixture.command, expected)
            .is_err());
    }
}

#[test]
fn future_exact_target_capture_prevents_fresh_commit_and_future_replay_disclosure() {
    let mut fixture = Fixture::single();
    let previous = base(&fixture);
    let r = previous.review;
    let group =
        prepare_measure_decision_capture(&Hasher, &r.actor, r.case_id, r.command, r.material)
            .unwrap()
            .into_group_capture(&Hasher, now() + Duration::seconds(1))
            .unwrap();
    fixture.history = crate::measure_dependency_support::judicial_inventory(&group).records;
    fixture.material.dependency_inventory.records = fixture.history.clone();
    fixture.command.target = reference(&group.measures[0]);
    fixture.material.target_head = fixture.command.target;
    let expected = confirmation(&fixture.review());
    let fresh = harness(fixture.store(), identity(fixture.actor.clone()));
    assert!(fresh
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command.clone(),
            expected
        )
        .is_err());

    let returned = fixture.operation(now() + Duration::seconds(2));
    let expected = confirmation(&returned.capture.review);
    let harness = harness(fixture.replay_store(returned), identity(fixture.actor));
    assert!(harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
    assert!(harness.events().is_empty());
}

#[test]
fn prepared_fresh_capture_cannot_precede_the_precommit_clock_observation() {
    let fixture = Fixture::single();
    let expected = confirmation(&fixture.review());
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| prepared.into_operation(now() - Duration::nanoseconds(1)));
    let harness = harness(store, identity(fixture.actor));
    assert!(harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
}

#[test]
fn final_clock_cannot_regress_even_when_commit_returns_an_older_raced_receipt() {
    let fixture = Fixture::single();
    let returned = fixture.operation(at());
    let expected = confirmation(&returned.capture.review);
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, _| Ok(returned));
    let harness = harness_with(
        store,
        identity(fixture.actor),
        Validator::default(),
        clock(vec![
            now(),
            now() + Duration::seconds(2),
            now() + Duration::seconds(1),
        ]),
    );
    assert!(harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
}
