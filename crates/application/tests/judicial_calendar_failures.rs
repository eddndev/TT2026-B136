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
fn malformed_preparation_or_foreign_base_never_reaches_commit() {
    for variant in 0..5 {
        let (identity, actor) = identity(Role::Owner, 1);
        let base = detail(actor.id, &command(), values("Calendar"));
        let cmd = replacement(&base);
        let mut prep = preparation(&base);
        match variant {
            0 => prep.calendar_id = JudicialCalendarId::new(),
            1 => prep.base.as_mut().unwrap().id = JudicialCalendarId::new(),
            2 => prep.initial_scope = None,
            3 => prep.initial_scope = Some(values("Other scope").scope().clone()),
            _ => prep.base.as_mut().unwrap().values_digest = Sha256Digest::from_array([0; 32]),
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _| Ok(prep));
        let (service, _) = service(store, identity);
        assert!(matches!(
            service.prepare("session", cmd),
            Err(ApplicationError::JudicialCalendar(
                JudicialCalendarError::StoredInconsistent(_)
            ))
        ));
    }
}
#[test]
fn replacement_cannot_change_the_first_revision_scope() {
    let (identity, actor) = identity(Role::Owner, 1);
    let base = detail(actor.id, &command(), values("Calendar"));
    let mut cmd = replacement(&base);
    if let JudicialCalendarChange::Replace { values: v, .. } = &mut cmd.change {
        *v = values("Other scope");
    }
    let prep = preparation(&base);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _| Ok(prep));
    let (service, _) = service(store, identity);
    assert!(matches!(
        service.prepare("session", cmd),
        Err(ApplicationError::JudicialCalendar(
            JudicialCalendarError::ScopeChangeForbidden
        ))
    ));
}
#[test]
fn stale_base_existing_root_and_terminal_retirement_are_rejected() {
    for variant in 0..3 {
        let (identity, actor) = identity(Role::Owner, 1);
        let first = detail(actor.id, &command(), values("Calendar"));
        let (cmd, base) = match variant {
            0 => {
                let second = detail(actor.id, &replacement(&first), first.values.clone());
                (replacement(&first), second)
            }
            1 => (
                JudicialCalendarCommand {
                    calendar_id: first.id,
                    ..command()
                },
                first,
            ),
            _ => {
                let retired = detail(actor.id, &retirement(&first), first.values.clone());
                (retirement(&retired), retired)
            }
        };
        let prep = preparation(&base);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _| Ok(prep));
        let (service, _) = service(store, identity);
        let error = service.prepare("session", cmd).unwrap_err();
        assert!(matches!(
            error,
            ApplicationError::JudicialCalendar(
                JudicialCalendarError::RevisionConflict | JudicialCalendarError::Retired
            )
        ));
    }
}
#[test]
fn exhausted_revision_is_rejected_before_store_access() {
    let (identity, _) = identity(Role::Owner, 1);
    let cmd = JudicialCalendarCommand {
        operation_id: JudicialCalendarOperationId::new(),
        calendar_id: JudicialCalendarId::new(),
        change: JudicialCalendarChange::Retire {
            expected_revision: JudicialCalendarRevision::new(u32::MAX).unwrap(),
            reason: JudicialCalendarReason::new("Retire").unwrap(),
        },
    };
    let (service, _) = service(MockStore::new(), identity);
    assert!(matches!(
        service.prepare("session", cmd),
        Err(ApplicationError::JudicialCalendar(
            JudicialCalendarError::RevisionExhausted
        ))
    ));
}
#[test]
fn commit_failure_is_propagated_without_retry() {
    for variant in 0..3 {
        let (identity, actor) = identity(Role::Owner, 2);
        let cmd = command();
        let result = detail(actor.id, &cmd, values("Calendar"));
        let prep = empty(&cmd);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _| Ok(prep));
        store.expect_commit().times(1).return_once(move |_, _| {
            Err(match variant {
                0 => JudicialCalendarError::OperationConflict.into(),
                1 => JudicialCalendarError::RevisionConflict.into(),
                _ => ApplicationError::Port("atomic audit failed".into()),
            })
        });
        let (service, _) = service(store, identity);
        assert!(service
            .submit("session", cmd, result.receipt.submission_digest)
            .is_err());
    }
}
#[test]
fn another_self_consistent_commit_cannot_replace_the_submitted_command() {
    for variant in 0..3 {
        let (identity, actor) = identity(Role::Owner, 2);
        let cmd = command();
        let result = detail(actor.id, &cmd, values("Calendar"));
        let prep = empty(&cmd);
        let mut different = cmd.clone();
        if variant == 0 {
            different.calendar_id = JudicialCalendarId::new();
        } else if variant == 1 {
            different.operation_id = JudicialCalendarOperationId::new();
        }
        let wrong = detail(
            if variant == 2 {
                UserId::new()
            } else {
                actor.id
            },
            &different,
            result.values.clone(),
        );
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _| Ok(prep));
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _| Ok(wrong));
        let (service, _) = service(store, identity);
        assert!(matches!(
            service.submit("session", cmd, result.receipt.submission_digest),
            Err(ApplicationError::JudicialCalendar(
                JudicialCalendarError::StoredInconsistent(_)
            ))
        ));
    }
}
