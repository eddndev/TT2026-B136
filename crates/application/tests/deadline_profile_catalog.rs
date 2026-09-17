#[allow(dead_code)]
mod case_support;
mod deadline_profile_catalog_support;
use application::{deadline_profiles::*, ApplicationError};
use deadline_profile_catalog_support::*;
use domain::{crypto::Sha256Digest, identity::Role};
const GLOBAL: DeadlineProfileCollection = DeadlineProfileCollection::Global;

#[test]
fn owner_preparation_is_stateless_and_binds_the_explicit_collection() {
    let (identity, actor) = case_support::identity(Role::Owner, 2);
    let command = command();
    let prep = empty(GLOBAL, &command);
    let expected = command.clone();
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |id, collection, command| {
            assert_eq!(id, actor.id);
            assert_eq!(collection, GLOBAL);
            assert_eq!(command, &expected);
            Ok(prep)
        });
    let (service, clock) = deadline_profile_catalog_support::service(store, identity);
    let draft = service.prepare("session", GLOBAL, command.clone()).unwrap();
    assert_eq!(draft.collection, GLOBAL);
    assert_eq!(draft.command, command);
    assert_eq!(draft.actor, actor.id);
    assert_eq!(draft.algorithm, DeadlineProfileAlgorithm::V1);
    assert_eq!(draft.result_revision.get(), 1);
    assert_eq!(draft.initial_scope, draft.definition.scope().clone());
    assert_eq!(clock.calls(), 0);
}

#[test]
fn submit_reprepares_and_commits_only_the_matching_digest_and_context() {
    let (identity, actor) = case_support::identity(Role::Owner, 2);
    let command = command();
    let prep = empty(GLOBAL, &command);
    let result = detail(actor.id, &command, definition(None));
    let expected = result.clone();
    let expected_command = command.clone();
    let digest = result.receipt.submission_digest;
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _| Ok(prep));
    store
        .expect_commit()
        .times(1)
        .return_once(move |id, prepared| {
            assert_eq!(id, actor.id);
            assert_eq!(id, prepared.actor());
            assert_eq!(prepared.collection(), GLOBAL);
            assert_eq!(prepared.command(), &expected_command);
            assert_eq!(prepared.definition(), &result.definition);
            assert_eq!(prepared.definition_digest(), result.definition_digest);
            assert_eq!(prepared.algorithm(), result.algorithm);
            assert_eq!(prepared.submission_digest(), digest);
            Ok(result)
        });
    let (service, clock) = deadline_profile_catalog_support::service(store, identity);
    assert_eq!(
        service.submit("session", GLOBAL, command, digest).unwrap(),
        expected
    );
    assert_eq!(clock.calls(), 0);
}

#[test]
fn denied_roles_do_not_reach_mutation_persistence_and_client_cannot_read() {
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        let (identity, _) = case_support::identity(role, 1);
        let (service, _) = deadline_profile_catalog_support::service(MockStore::new(), identity);
        assert!(matches!(
            service.prepare("session", GLOBAL, command()),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    let (identity, _) = case_support::identity(Role::Client, 1);
    let (service, _) = deadline_profile_catalog_support::service(MockStore::new(), identity);
    assert!(matches!(
        service.get("session", GLOBAL, DeadlineProfileId::new(), None),
        Err(ApplicationError::PermissionDenied)
    ));
}

#[test]
fn submission_mismatch_never_reauthenticates_or_commits() {
    let (identity, _) = case_support::identity(Role::Owner, 1);
    let command = command();
    let prep = empty(GLOBAL, &command);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _| Ok(prep));
    let (service, _) = deadline_profile_catalog_support::service(store, identity);
    assert!(matches!(
        service.submit(
            "session",
            GLOBAL,
            command,
            Sha256Digest::from_array([0x77; 32])
        ),
        Err(ApplicationError::DeadlineProfile(
            DeadlineProfileError::SubmissionMismatch
        ))
    ));
}

#[test]
fn retirement_preserves_definition_algorithm_and_terminal_status() {
    let (identity, actor) = case_support::identity(Role::Owner, 2);
    let base = detail(actor.id, &command(), definition(None));
    let command = retirement(&base);
    let prep = preparation(GLOBAL, &base);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _| Ok(prep));
    let (service, clock) = deadline_profile_catalog_support::service(store, identity);
    let draft = service.prepare("session", GLOBAL, command).unwrap();
    assert_eq!(draft.definition, base.definition);
    assert_eq!(draft.definition_digest, base.definition_digest);
    assert_eq!(draft.algorithm, base.algorithm);
    assert_eq!(draft.result_revision.get(), 2);
    assert_eq!(clock.calls(), 0);

    let (identity, actor) = case_support::identity(Role::Owner, 1);
    let base = detail(actor.id, &retirement(&base), base.definition.clone());
    let command = replacement(&base);
    let prep = preparation(GLOBAL, &base);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _| Ok(prep));
    let (service, _) = deadline_profile_catalog_support::service(store, identity);
    assert!(matches!(
        service.prepare("session", GLOBAL, command),
        Err(ApplicationError::DeadlineProfile(
            DeadlineProfileError::Retired
        ))
    ));
}
