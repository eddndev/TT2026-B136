use application::identity::invitations::*;
use application::identity::password_reset::ResetDeliveryOutcome;
use application::ApplicationError;
use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::Sha256Digest,
};
use std::sync::{Arc, Mutex};
use time::Duration;
use zeroize::Zeroizing;

#[path = "delivery_worker_mocks.rs"]
mod mocks;
use mocks::*;

struct State {
    now: OffsetDateTime,
    available: Option<InvitationDeliveryLease>,
    claimed: Option<InvitationDeliveryLease>,
    claims: usize,
    decrypts: usize,
    sent: usize,
    finishes: Vec<InvitationDeliveryFinish>,
    outcome: ResetDeliveryOutcome,
    delivery_error: bool,
    decrypt_error: bool,
    claim_error: bool,
    finish_error: bool,
    finish_accepts: bool,
    decrypt_delay: Duration,
}

fn harness() -> (InvitationDeliveryWorker, Arc<Mutex<State>>) {
    let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
    let state = Arc::new(Mutex::new(State {
        now,
        available: Some(InvitationDeliveryLease {
            id: InvitationId::from_uuid(Uuid::from_u128(1)),
            revision: 2,
            delivery_id: Uuid::from_u128(3),
            generation: 4,
            lease_id: Uuid::nil(),
            email: "recipient@example.test".into(),
            protected_token: vec![7, 8, 9],
            expires_at: now + Duration::hours(24),
            lease_until: now + Duration::seconds(30),
            provider_deadline: now + Duration::hours(24),
        }),
        claimed: None,
        claims: 0,
        decrypts: 0,
        sent: 0,
        finishes: Vec::new(),
        outcome: ResetDeliveryOutcome::Accepted,
        delivery_error: false,
        decrypt_error: false,
        claim_error: false,
        finish_error: false,
        finish_accepts: true,
        decrypt_delay: Duration::ZERO,
    }));
    let mut store = MockStore::new();
    let shared = state.clone();
    store.expect_claim_delivery().returning(move |claim| {
        let mut state = shared.lock().unwrap();
        state.claims += 1;
        assert_eq!(claim.at, state.now);
        assert!(!claim.lease_id.is_nil());
        assert_eq!(claim.lease_seconds, 30);
        assert_eq!(claim.provider_window_seconds, 86_400);
        if state.claim_error {
            return Err(ApplicationError::Port("private-claim-detail".into()));
        }
        let mut lease = state.available.clone();
        if let Some(value) = &mut lease {
            value.lease_id = claim.lease_id;
        }
        state.claimed = lease.clone();
        Ok(lease)
    });
    let shared = state.clone();
    store.expect_finish_delivery().returning(move |finish| {
        let mut state = shared.lock().unwrap();
        let claimed = state.claimed.as_ref().unwrap();
        assert_eq!(finish.id, claimed.id);
        assert_eq!(finish.revision, claimed.revision);
        assert_eq!(finish.generation, claimed.generation);
        assert_eq!(finish.lease_id, claimed.lease_id);
        assert_eq!(finish.at, state.now);
        state.finishes.push(finish);
        if state.finish_error {
            return Err(ApplicationError::Port("private-finish-detail".into()));
        }
        Ok(state.finish_accepts)
    });
    let mut protector = MockProtector::new();
    let shared = state.clone();
    protector
        .expect_expose()
        .returning(move |context, protected| {
            let mut state = shared.lock().unwrap();
            state.decrypts += 1;
            let claimed = state.claimed.as_ref().unwrap();
            assert_eq!(
                context,
                InvitationProtectionContext {
                    purpose: InvitationPurpose::Delivery,
                    id: claimed.id,
                    revision: claimed.revision,
                }
            );
            assert_eq!(protected, claimed.protected_token);
            let delay = state.decrypt_delay;
            state.now += delay;
            if state.decrypt_error {
                return Err(ApplicationError::Port("private-token-detail".into()));
            }
            Ok(Zeroizing::new(vec![0x42; 32]))
        });
    let mut delivery = MockDelivery::new();
    let shared = state.clone();
    delivery.expect_deliver().returning(move |lease, token| {
        let mut state = shared.lock().unwrap();
        state.sent += 1;
        assert_eq!(
            lease.delivery_id,
            state.claimed.as_ref().unwrap().delivery_id
        );
        assert_eq!(token.as_slice(), &[0x42; 32]);
        if state.delivery_error {
            return Err(ApplicationError::Port("private-provider-detail".into()));
        }
        Ok(state.outcome)
    });
    let mut clock = MockTime::new();
    let shared = state.clone();
    clock
        .expect_now()
        .returning(move || shared.lock().unwrap().now);
    let worker = InvitationDeliveryWorker::new(
        Arc::new(store),
        Arc::new(protector),
        Arc::new(delivery),
        Arc::new(clock),
    );
    (worker, state)
}

#[test]
fn empty_claim_does_no_crypto_or_delivery() {
    let (worker, state) = harness();
    state.lock().unwrap().available = None;
    assert_eq!(worker.run_once().unwrap(), InvitationDeliveryRun::Idle);
    let state = state.lock().unwrap();
    assert_eq!((state.claims, state.decrypts, state.sent), (1, 0, 0));
    assert!(state.finishes.is_empty());
}

#[test]
fn provider_outcomes_are_recorded_once_with_original_fencing() {
    for (provider, expected) in [
        (
            ResetDeliveryOutcome::Accepted,
            InvitationDeliveryState::Sent,
        ),
        (
            ResetDeliveryOutcome::DefinitelyRejected,
            InvitationDeliveryState::Rejected,
        ),
        (
            ResetDeliveryOutcome::Uncertain,
            InvitationDeliveryState::Uncertain,
        ),
    ] {
        let (worker, state) = harness();
        state.lock().unwrap().outcome = provider;
        assert_eq!(
            worker.run_once().unwrap(),
            InvitationDeliveryRun::Recorded(expected)
        );
        let state = state.lock().unwrap();
        assert_eq!((state.claims, state.decrypts, state.sent), (1, 1, 1));
        assert_eq!(state.finishes.len(), 1);
        assert_eq!(state.finishes[0].state, expected);
        assert_eq!(
            state.finishes[0].retry_at,
            if expected == InvitationDeliveryState::Uncertain {
                Some(state.now + Duration::seconds(60))
            } else {
                None
            }
        );
    }
}

#[test]
fn transport_error_preserves_uncertainty_without_exposing_details() {
    let (worker, state) = harness();
    state.lock().unwrap().delivery_error = true;
    let result = worker.run_once().unwrap();
    assert_eq!(
        result,
        InvitationDeliveryRun::Recorded(InvitationDeliveryState::Uncertain)
    );
    assert!(!format!("{result:?}").contains("private-provider-detail"));
    let state = state.lock().unwrap();
    assert_eq!(state.sent, 1);
    assert_eq!(
        state.finishes[0].retry_at,
        Some(state.now + Duration::seconds(60))
    );
}

#[test]
fn elapsed_lease_provider_deadline_or_expiry_prevents_delivery() {
    for field in ["lease", "provider", "expiry"] {
        let (worker, state) = harness();
        {
            let mut state = state.lock().unwrap();
            let now = state.now;
            let lease = state.available.as_mut().unwrap();
            match field {
                "lease" => lease.lease_until = now,
                "provider" => lease.provider_deadline = now,
                _ => lease.expires_at = now,
            }
        }
        assert_eq!(worker.run_once().unwrap(), InvitationDeliveryRun::Stale);
        let state = state.lock().unwrap();
        assert_eq!((state.decrypts, state.sent), (0, 0));
        assert!(state.finishes.is_empty());
    }
}

#[test]
fn time_is_rechecked_after_decryption_before_provider_io() {
    let (worker, state) = harness();
    state.lock().unwrap().decrypt_delay = Duration::seconds(30);
    assert_eq!(worker.run_once().unwrap(), InvitationDeliveryRun::Stale);
    let state = state.lock().unwrap();
    assert_eq!((state.decrypts, state.sent), (1, 0));
    assert!(state.finishes.is_empty());
}

#[test]
fn uncertain_retry_must_fit_before_original_provider_deadline_and_expiry() {
    for provider_boundary in [true, false] {
        let (worker, state) = harness();
        {
            let mut state = state.lock().unwrap();
            state.outcome = ResetDeliveryOutcome::Uncertain;
            let boundary = state.now + Duration::seconds(60);
            let lease = state.available.as_mut().unwrap();
            if provider_boundary {
                lease.provider_deadline = boundary;
            } else {
                lease.expires_at = boundary;
            }
        }
        assert_eq!(
            worker.run_once().unwrap(),
            InvitationDeliveryRun::Recorded(InvitationDeliveryState::Uncertain)
        );
        assert_eq!(state.lock().unwrap().finishes[0].retry_at, None);
    }
}

#[test]
fn stale_finish_never_confirms_a_recorded_delivery() {
    let (worker, state) = harness();
    state.lock().unwrap().finish_accepts = false;
    assert_eq!(worker.run_once().unwrap(), InvitationDeliveryRun::Stale);
    let state = state.lock().unwrap();
    assert_eq!(state.sent, 1);
    assert_eq!(state.finishes.len(), 1);
}

#[test]
fn decryption_failure_is_recorded_without_delivery_or_secret_details() {
    let (worker, state) = harness();
    state.lock().unwrap().decrypt_error = true;
    let result = worker.run_once().unwrap();
    assert_eq!(
        result,
        InvitationDeliveryRun::Recorded(InvitationDeliveryState::Uncertain)
    );
    assert!(!format!("{result:?}").contains("private-token-detail"));
    let state = state.lock().unwrap();
    assert_eq!((state.decrypts, state.sent), (1, 0));
    assert_eq!(state.finishes[0].state, InvitationDeliveryState::Uncertain);
}

#[test]
fn store_errors_remain_errors_without_private_port_diagnostics() {
    for claim_error in [true, false] {
        let (worker, state) = harness();
        {
            let mut state = state.lock().unwrap();
            state.claim_error = claim_error;
            state.finish_error = !claim_error;
        }
        let error = worker.run_once().unwrap_err();
        assert!(!format!("{error:?} {error}").contains("private-"));
    }
}
