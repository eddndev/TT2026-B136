#[allow(dead_code)]
mod case_support;
mod deadline_profile_catalog_support;
use application::{deadline_profiles::*, ApplicationError};
use deadline_profile_catalog_support::*;
use domain::{crypto::Sha256Digest, identity::Role};
const GLOBAL: DeadlineProfileCollection = DeadlineProfileCollection::Global;

#[test]
fn scope_changes_and_wrong_authorization_collections_cannot_publish_or_replace() {
    use domain::cases::CaseId;
    for case in [None, Some(CaseId::new())] {
        let (identity, actor) = case_support::identity(Role::Owner, 1);
        let collection = case
            .map(DeadlineProfileCollection::ForCase)
            .unwrap_or(GLOBAL);
        let base = detail(actor.id, &command(), definition(case));
        let mut changed = input(case);
        changed.scope = if case.is_some() {
            DeadlineProfileScope::Case(CaseId::new())
        } else {
            altered_global_scope()
        };
        let mut command = replacement(&base);
        if let DeadlineProfileChange::Replace { definition, .. } = &mut command.change {
            *definition = DeadlineProfileDefinition::new(changed).unwrap();
        }
        let prep = preparation(collection, &base);
        let mut store = MockStore::new();
        if case.is_none() {
            store
                .expect_prepare()
                .times(1)
                .return_once(move |_, _, _| Ok(prep));
        }
        let (service, _) = deadline_profile_catalog_support::service(store, identity);
        assert!(matches!(
            service.prepare("session", collection, command),
            Err(ApplicationError::DeadlineProfile(
                DeadlineProfileError::ScopeChangeForbidden
            ))
        ));
    }
    let (identity, _) = case_support::identity(Role::Owner, 1);
    let (service, _) = deadline_profile_catalog_support::service(MockStore::new(), identity);
    assert!(matches!(
        service.prepare(
            "session",
            DeadlineProfileCollection::ForCase(CaseId::new()),
            command()
        ),
        Err(ApplicationError::DeadlineProfile(
            DeadlineProfileError::ScopeChangeForbidden
        ))
    ));
}

#[test]
fn preparation_rejects_forged_base_receipts_or_changed_returned_context() {
    for mutation in 0..3 {
        let (identity, actor) = case_support::identity(Role::Owner, 1);
        let base = detail(actor.id, &command(), definition(None));
        let command = replacement(&base);
        let mut prep = preparation(GLOBAL, &base);
        match mutation {
            0 => prep.collection = DeadlineProfileCollection::ForCase(domain::cases::CaseId::new()),
            1 => {
                prep.base.as_mut().unwrap().definition_digest = Sha256Digest::from_array([0xAB; 32])
            }
            _ => prep.initial_scope = None,
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _| Ok(prep));
        let (service, _) = deadline_profile_catalog_support::service(store, identity);
        assert!(matches!(
            service.prepare("session", GLOBAL, command),
            Err(ApplicationError::DeadlineProfile(
                DeadlineProfileError::StoredInconsistent(_)
            ))
        ));
    }
}

#[test]
fn stale_base_and_exhausted_revision_never_commit() {
    let (identity, actor) = case_support::identity(Role::Owner, 1);
    let first = detail(actor.id, &command(), definition(None));
    let changed = detail(actor.id, &replacement(&first), first.definition.clone());
    let command = replacement(&first);
    let prep = preparation(GLOBAL, &changed);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _| Ok(prep));
    let (service, _) = deadline_profile_catalog_support::service(store, identity);
    assert!(matches!(
        service.prepare("session", GLOBAL, command),
        Err(ApplicationError::DeadlineProfile(
            DeadlineProfileError::RevisionConflict
        ))
    ));
    let command = DeadlineProfileCommand {
        operation_id: DeadlineProfileOperationId::new(),
        profile_id: first.id,
        change: DeadlineProfileChange::Retire {
            expected_revision: DeadlineProfileRevision::new(u32::MAX).unwrap(),
            reason: text("Retire"),
        },
    };
    assert!(DeadlineProfileRevision::new(0).is_err());
    assert!(matches!(
        command.result_revision(),
        Err(ApplicationError::DeadlineProfile(
            DeadlineProfileError::RevisionExhausted
        ))
    ));
}

#[test]
fn committed_identity_or_definition_tampering_is_never_reported_as_success() {
    for mutation in 0..4 {
        let (identity, actor) = case_support::identity(Role::Owner, 2);
        let command = command();
        let prep = empty(GLOBAL, &command);
        let mut result = detail(actor.id, &command, definition(None));
        let digest = result.receipt.submission_digest;
        match mutation {
            0 => result.id = DeadlineProfileId::new(),
            1 => result.definition = definition(Some(domain::cases::CaseId::new())),
            2 => result.recorded_by.id = domain::identity::UserId::new(),
            _ => {
                let mut other = command.clone();
                other.operation_id = DeadlineProfileOperationId::new();
                result = detail(actor.id, &other, definition(None));
            }
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _| Ok(prep));
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _| Ok(result));
        let (service, _) = deadline_profile_catalog_support::service(store, identity);
        assert!(matches!(
            service.submit("session", GLOBAL, command, digest),
            Err(ApplicationError::DeadlineProfile(
                DeadlineProfileError::StoredInconsistent(_)
            ))
        ));
    }
}

#[test]
fn expired_or_replaced_session_prevents_commit_after_preparation() {
    for replaced in [false, true] {
        let actor = domain::identity::UserId::new();
        let command = command();
        let prep = empty(GLOBAL, &command);
        let digest = detail(actor, &command, definition(None))
            .receipt
            .submission_digest;
        let mut identity = MockIdentity::new();
        let mut sequence = mockall::Sequence::new();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                Ok(application::identity::Principal {
                    id: actor,
                    email: "owner@example.com".into(),
                    role: Role::Owner,
                })
            });
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                if replaced {
                    Ok(application::identity::Principal {
                        id: domain::identity::UserId::new(),
                        email: "other@example.com".into(),
                        role: Role::Owner,
                    })
                } else {
                    Err(ApplicationError::InvalidSession)
                }
            });
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _| Ok(prep));
        let (service, _) = deadline_profile_catalog_support::service(store, identity);
        assert!(matches!(
            service.submit("session", GLOBAL, command, digest),
            Err(ApplicationError::InvalidSession)
        ));
    }
}
