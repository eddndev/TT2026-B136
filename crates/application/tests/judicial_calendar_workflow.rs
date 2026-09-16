#[allow(dead_code)]
mod case_support;
mod judicial_calendar_support;
use application::{judicial_calendars::*, ApplicationError};
use domain::{
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
use judicial_calendar_support::*;

#[test]
fn preparation_is_stateless_and_never_reserves_a_capture_time() {
    let (identity, actor) = identity(Role::Owner, 2);
    let cmd = command();
    let prep = empty(&cmd);
    let mut store = MockStore::new();
    store.expect_prepare().times(1).return_once(move |id, c| {
        assert_eq!(id, actor.id);
        assert_eq!(c.calendar_id, prep.calendar_id);
        Ok(prep)
    });
    let (service, clock) = service(store, identity);
    let draft = service.prepare("session", cmd.clone()).unwrap();
    assert_eq!(draft.command, cmd);
    assert_eq!(draft.actor, actor.id);
    assert_eq!(draft.result_revision.get(), 1);
    assert_eq!(draft.initial_scope, draft.values.scope().clone());
    assert_eq!(clock.calls(), 0);
}
#[test]
fn submit_reprepares_and_commits_the_exact_validated_operation() {
    let (identity, actor) = identity(Role::Owner, 2);
    let cmd = command();
    let prep = empty(&cmd);
    let result = detail(actor.id, &cmd, values("Calendar"));
    let expected = result.clone();
    let digest = result.receipt.submission_digest;
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _| Ok(prep));
    store
        .expect_commit()
        .times(1)
        .return_once(move |id, prepared| {
            assert_eq!(id, prepared.actor());
            assert_eq!(id, actor.id);
            assert_eq!(prepared.command(), &cmd);
            assert_eq!(prepared.values(), &result.values);
            assert_eq!(prepared.values_digest(), result.values_digest);
            assert_eq!(prepared.submission_digest(), digest);
            Ok(result)
        });
    let (service, clock) = service(store, identity);
    assert_eq!(
        service
            .submit("session", expected_command(&expected), digest)
            .unwrap(),
        expected
    );
    assert_eq!(clock.calls(), 0);
}
fn expected_command(d: &JudicialCalendarDetail) -> JudicialCalendarCommand {
    JudicialCalendarCommand {
        operation_id: d.receipt.operation_id,
        calendar_id: d.id,
        change: JudicialCalendarChange::Publish {
            values: d.values.clone(),
        },
    }
}
#[test]
fn retirement_copies_exact_base_values_without_a_new_source_lookup() {
    let (identity, actor) = identity(Role::Owner, 2);
    let base = detail(actor.id, &command(), values("Historical"));
    let cmd = retirement(&base);
    let prep = preparation(&base);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _| Ok(prep));
    let (service, clock) = service(store, identity);
    let draft = service.prepare("session", cmd).unwrap();
    assert_eq!(draft.values, base.values);
    assert_eq!(draft.values_digest, base.values_digest);
    assert_eq!(draft.result_revision.get(), 2);
    assert_eq!(clock.calls(), 0);
}
#[test]
fn identical_values_with_another_operation_produce_an_explicit_revision() {
    let (identity, actor) = identity(Role::Owner, 2);
    let base = detail(actor.id, &command(), values("Calendar"));
    let cmd = replacement(&base);
    let prep = preparation(&base);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _| Ok(prep));
    let (service, _) = service(store, identity);
    let draft = service.prepare("session", cmd).unwrap();
    assert_eq!(draft.values_digest, base.values_digest);
    assert_ne!(draft.submission_digest, base.receipt.submission_digest);
    assert_eq!(draft.result_revision.get(), 2);
}
#[test]
fn denied_roles_never_reach_calendar_persistence() {
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        let (identity, _) = identity(role, 1);
        let (service, clock) = service(MockStore::new(), identity);
        assert!(matches!(
            service.prepare("session", command()),
            Err(ApplicationError::PermissionDenied)
        ));
        assert_eq!(clock.calls(), 0);
    }
    let (identity, _) = identity(Role::Client, 1);
    let (service, _) = service(MockStore::new(), identity);
    assert!(matches!(
        service.get("session", JudicialCalendarId::new(), None),
        Err(ApplicationError::PermissionDenied)
    ));
}
#[test]
fn submission_mismatch_does_not_reauthenticate_or_commit() {
    let (identity, _) = identity(Role::Owner, 1);
    let cmd = command();
    let prep = empty(&cmd);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _| Ok(prep));
    let (service, _) = service(store, identity);
    assert!(matches!(
        service.submit("session", cmd, Sha256Digest::from_array([0x55; 32])),
        Err(ApplicationError::JudicialCalendar(
            JudicialCalendarError::SubmissionMismatch
        ))
    ));
}
#[test]
fn session_expiration_or_actor_replacement_prevents_commit() {
    for replace_actor in [false, true] {
        let actor = UserId::new();
        let cmd = command();
        let result = detail(actor, &cmd, values("Calendar"));
        let prep = empty(&cmd);
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
                if replace_actor {
                    Ok(application::identity::Principal {
                        id: UserId::new(),
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
            .return_once(move |_, _| Ok(prep));
        let (service, _) = service(store, identity);
        assert!(matches!(
            service.submit("session", cmd, result.receipt.submission_digest),
            Err(ApplicationError::InvalidSession)
        ));
    }
}
