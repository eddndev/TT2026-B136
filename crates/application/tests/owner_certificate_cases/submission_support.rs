use std::sync::{Arc, Mutex};

use application::identity::owner_certificates::{
    OwnerRegistrationContext, OwnerRegistrationSubmission,
};
use uuid::Uuid;

use crate::support::*;

pub fn submission() -> OwnerRegistrationSubmission {
    OwnerRegistrationSubmission {
        statement: statement().canonical_bytes().to_vec(),
        certificate_der: certificate().der,
        signature: signature(),
    }
}

pub fn harness() -> Harness {
    let mut harness = Harness::new();
    harness.verifier = MockVerifier::new();
    let observed = harness.state.clone();
    harness
        .verifier
        .expect_inspect_certificate()
        .returning(move |bytes| {
            assert_eq!(bytes, certificate().der);
            observed.lock().unwrap().calls.push("inspect");
            Ok(certificate())
        });
    harness
}

pub fn replace_store(
    harness: &mut Harness,
    expected_binding: Uuid,
    on_find: impl Fn(&mut State) + Send + Sync + 'static,
    load: impl Fn(&mut State) -> OwnerRegistrationContext + Send + Sync + 'static,
) {
    let mut store = MockStore::new();
    let observed = harness.state.clone();
    store.expect_find().returning(move |actor, binding| {
        assert_eq!((actor, binding), (principal().id, expected_binding));
        let mut state = observed.lock().unwrap();
        state.calls.push("find");
        on_find(&mut state);
        Ok(state.found.clone())
    });
    let observed = harness.state.clone();
    store.expect_load_registration().returning(move |actor| {
        assert_eq!(actor, principal().id);
        let mut state = observed.lock().unwrap();
        state.calls.push("load_registration");
        Ok(load(&mut state))
    });
    harness.store = store;
}

pub fn count(observed: &Arc<Mutex<State>>, call: &str) -> usize {
    observed
        .lock()
        .unwrap()
        .calls
        .iter()
        .filter(|value| **value == call)
        .count()
}

pub fn no_crypto_or_commit(observed: &Arc<Mutex<State>>) {
    assert_eq!(count(observed, "verify"), 0);
    assert_eq!(count(observed, "commit_registration"), 0);
}
