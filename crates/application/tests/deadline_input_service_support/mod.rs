use crate::case_support::MockIdentity;
use application::{
    deadline_inputs::{
        DeadlineInputMaterial, DeadlineInputRequest, DeadlineInputService, DeadlineInputStore,
    },
    identity::Principal,
    ApplicationError,
};
use domain::identity::{Role, UserId};
use mockall::{mock, Sequence};
use std::sync::Arc;

mock! {
    pub Store {}
    impl DeadlineInputStore for Store {
        fn load(&self, actor: UserId, request: &DeadlineInputRequest)
            -> Result<DeadlineInputMaterial, ApplicationError>;
    }
}

pub fn principal(role: Role) -> Principal {
    Principal {
        id: crate::deadline_input_support::actor(),
        email: "reader@example.com".into(),
        role,
    }
}

pub fn expect_identity(
    identity: &mut MockIdentity,
    sequence: &mut Sequence,
    result: Result<Principal, ApplicationError>,
) {
    identity
        .expect_authenticate()
        .withf(|token| token == "session")
        .times(1)
        .in_sequence(sequence)
        .return_once(move |_| result);
}

pub fn expect_load(
    store: &mut MockStore,
    sequence: &mut Sequence,
    actor: UserId,
    request: DeadlineInputRequest,
    result: Result<DeadlineInputMaterial, ApplicationError>,
) {
    store
        .expect_load()
        .withf(move |observed_actor, observed_request| {
            *observed_actor == actor && observed_request == &request
        })
        .times(1)
        .in_sequence(sequence)
        .return_once(move |_, _| result);
}

pub fn service(store: MockStore, identity: MockIdentity) -> DeadlineInputService {
    DeadlineInputService::new(
        Arc::new(store),
        Arc::new(identity),
        crate::deadline_input_support::hasher(),
    )
}
