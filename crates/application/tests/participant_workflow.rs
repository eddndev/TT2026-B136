#[allow(dead_code)]
mod case_support;
mod participant_support;

use application::participants::{
    DirectoryStatus, ParticipantHistoryPage, ParticipantHistoryQuery, ParticipantId,
    ParticipantPage, ParticipantQuery, ParticipantRevision, ParticipantStatusFilter,
    ParticipantWorkflow,
};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::identity::{Role, UserId};
use participant_support::{identity, instant, service, snapshot, values, MockIdentity, MockStore};

fn list_query() -> ParticipantQuery {
    ParticipantQuery::new(
        2,
        Some(ParticipantId::from_uuid(uuid::Uuid::from_u128(7))),
        Some("A%_"),
        Some("Defensa"),
        ParticipantStatusFilter::All,
    )
    .unwrap()
}

fn history_query() -> ParticipantHistoryQuery {
    ParticipantHistoryQuery::new(2, Some(8)).unwrap()
}
fn revision() -> ParticipantRevision {
    ParticipantRevision::new(7).unwrap()
}

#[test]
fn clients_and_revoked_sessions_reach_neither_storage_nor_clock() {
    for revoked in [false, true] {
        let credentials = if revoked {
            let mut mock = MockIdentity::new();
            mock.expect_authenticate()
                .times(6)
                .returning(|_| Err(ApplicationError::InvalidSession));
            mock
        } else {
            identity(Role::Client, 6).0
        };
        let (workflow, clock) = service(MockStore::new(), credentials);
        let case = CaseId::new();
        let id = ParticipantId::new();
        for error in [
            workflow
                .create("session", case, "\n", "", None, None)
                .unwrap_err(),
            workflow
                .replace("session", case, id, revision(), values())
                .unwrap_err(),
            workflow
                .change_status("session", case, id, revision(), DirectoryStatus::Archived)
                .unwrap_err(),
            workflow.list("session", case, list_query()).unwrap_err(),
            workflow.get("session", case, id).unwrap_err(),
            workflow
                .history("session", case, id, history_query())
                .unwrap_err(),
        ] {
            assert!(if revoked {
                matches!(error, ApplicationError::InvalidSession)
            } else {
                matches!(error, ApplicationError::PermissionDenied)
            });
        }
        assert_eq!(clock.calls(), 0);
    }
}

#[test]
fn paralegals_cannot_create_replace_or_change_status() {
    let (workflow, clock) = service(MockStore::new(), identity(Role::Paralegal, 3).0);
    let case = CaseId::new();
    let id = ParticipantId::new();
    for error in [
        workflow
            .create("session", case, "Ana", "Defensa", None, None)
            .unwrap_err(),
        workflow
            .replace("session", case, id, revision(), values())
            .unwrap_err(),
        workflow
            .change_status("session", case, id, revision(), DirectoryStatus::Archived)
            .unwrap_err(),
    ] {
        assert!(matches!(error, ApplicationError::PermissionDenied));
    }
    assert_eq!(clock.calls(), 0);
}

#[test]
fn creation_normalizes_forces_active_and_preserves_commit_provenance() {
    for role in [Role::Owner, Role::Litigator] {
        let (credentials, actor) = identity(role, 1);
        let case = CaseId::new();
        let user = actor.id;
        let mut store = MockStore::new();
        store
            .expect_create()
            .times(1)
            .withf(move |who, scope, id, input, at| {
                *who == user
                    && *scope == case
                    && id.as_uuid().get_version_num() == 4
                    && *input == values()
                    && *at == instant()
            })
            .return_once(move |_, scope, id, _, _| Ok(snapshot(scope, id, user, 1)));
        let (workflow, clock) = service(store, credentials);
        let result = workflow
            .create(
                "session",
                case,
                " Ana ",
                "\u{2003}Defensa ",
                Some(" Despacho "),
                Some(" "),
            )
            .unwrap();
        assert_eq!(result, snapshot(case, result.id, user, 1));
        assert_ne!(result.changed_by.email, actor.email);
        assert_eq!(clock.calls(), 1);
    }
}

#[test]
fn invalid_creation_values_fail_before_storage_and_clock() {
    let (workflow, clock) = service(MockStore::new(), identity(Role::Owner, 1).0);
    assert!(matches!(
        workflow.create("session", CaseId::new(), "\nAna", "Defensa", None, None),
        Err(ApplicationError::Domain(
            domain::DomainError::InvalidParticipantValues { .. }
        ))
    ));
    assert_eq!(clock.calls(), 0);
}

#[test]
fn full_replacement_forwards_expected_and_returns_the_committed_snapshot() {
    for role in [Role::Owner, Role::Litigator] {
        let (credentials, actor) = identity(role, 1);
        let user = actor.id;
        let case = CaseId::new();
        let id = ParticipantId::new();
        let input = values().with_directory_status(DirectoryStatus::Archived);
        let expected_input = input.clone();
        let mut expected = snapshot(case, id, user, 8);
        expected.values = input.clone();
        let returned = expected.clone();
        let mut store = MockStore::new();
        store
            .expect_replace()
            .times(1)
            .withf(move |who, scope, participant, head, value, at| {
                *who == user
                    && *scope == case
                    && *participant == id
                    && *head == revision()
                    && *value == expected_input
                    && *at == instant()
            })
            .return_once(move |_, _, _, _, _, _| Ok(returned));
        let (workflow, clock) = service(store, credentials);
        assert_eq!(
            workflow
                .replace("session", case, id, revision(), input)
                .unwrap(),
            expected
        );
        assert_eq!(clock.calls(), 1);
    }
}

#[test]
fn status_change_uses_the_atomic_port_and_preserves_its_latest_text_values() {
    let (credentials, actor) = identity(Role::Litigator, 1);
    let user = actor.id;
    let case = CaseId::new();
    let id = ParticipantId::new();
    let mut expected = snapshot(case, id, user, 8);
    expected.values = application::participants::ParticipantValues::new(
        "Concurrent name",
        "Witness",
        Some("Current organization"),
        Some("Unchanged legal text"),
        DirectoryStatus::Archived,
    )
    .unwrap();
    let returned = expected.clone();
    let mut store = MockStore::new();
    store
        .expect_change_status()
        .times(1)
        .withf(move |who, scope, participant, head, status, at| {
            *who == user
                && *scope == case
                && *participant == id
                && *head == revision()
                && *status == DirectoryStatus::Archived
                && *at == instant()
        })
        .return_once(move |_, _, _, _, _, _| Ok(returned.into()));
    let (workflow, clock) = service(store, credentials);
    assert_eq!(
        workflow
            .change_status("session", case, id, revision(), DirectoryStatus::Archived)
            .unwrap(),
        expected.into()
    );
    assert_eq!(clock.calls(), 1);
}

#[test]
fn paralegal_reads_forward_scope_cursors_and_captured_history() {
    let (credentials, actor) = identity(Role::Paralegal, 3);
    let user = actor.id;
    let case = CaseId::new();
    let id = ParticipantId::new();
    let expected = snapshot(case, id, UserId::new(), 7);
    let page = ParticipantPage {
        participants: vec![expected.clone().into()],
        has_more: true,
        next_after_id: Some(id),
    };
    let history = ParticipantHistoryPage {
        revisions: vec![expected.clone().into()],
        has_more: true,
        next_before_revision: Some(revision()),
    };
    let returned = expected.clone();
    let returned_page = page.clone();
    let returned_history = history.clone();
    let mut store = MockStore::new();
    store
        .expect_get()
        .times(1)
        .withf(move |who, scope, participant, at| {
            *who == user && *scope == case && *participant == id && *at == instant()
        })
        .return_once(move |_, _, _, _| Ok(returned.into()));
    store
        .expect_list()
        .times(1)
        .withf(move |who, scope, query, at| {
            *who == user && *scope == case && *query == list_query() && *at == instant()
        })
        .return_once(move |_, _, _, _| Ok(returned_page));
    store
        .expect_history()
        .times(1)
        .withf(move |who, scope, participant, query, at| {
            *who == user
                && *scope == case
                && *participant == id
                && *query == history_query()
                && *at == instant()
        })
        .return_once(move |_, _, _, _, _| Ok(returned_history));
    let (workflow, clock) = service(store, credentials);
    assert_eq!(workflow.get("session", case, id).unwrap(), expected.into());
    assert_eq!(workflow.list("session", case, list_query()).unwrap(), page);
    assert_eq!(
        workflow
            .history("session", case, id, history_query())
            .unwrap(),
        history
    );
    assert_eq!(clock.calls(), 3);
}

#[test]
fn storage_failures_preserve_their_meaning_without_a_followup_read() {
    let (credentials, _) = identity(Role::Owner, 7);
    let mut store = MockStore::new();
    store
        .expect_create()
        .times(1)
        .returning(|_, _, _, _, _| Err(ApplicationError::Port("commit failed".into())));
    let mut sequence = mockall::Sequence::new();
    for error in [
        ApplicationError::ParticipantRevisionConflict,
        ApplicationError::ParticipantRevisionExhausted,
    ] {
        store
            .expect_replace()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_, _, _, _, _, _| Err(error));
    }
    store
        .expect_change_status()
        .times(1)
        .returning(|_, _, _, _, _, _| {
            Err(ApplicationError::StoredParticipantInconsistent(
                "digest".into(),
            ))
        });
    store
        .expect_get()
        .times(1)
        .returning(|_, _, _, _| Err(ApplicationError::ParticipantNotFound));
    store
        .expect_list()
        .times(1)
        .returning(|_, _, _, _| Err(ApplicationError::CaseNotFound));
    store
        .expect_history()
        .times(1)
        .returning(|_, _, _, _, _| Err(ApplicationError::Port("audit failed".into())));
    let (workflow, clock) = service(store, credentials);
    let case = CaseId::new();
    let id = ParticipantId::new();
    assert!(matches!(
        workflow.create("session", case, "A", "R", None, None),
        Err(ApplicationError::Port(_))
    ));
    assert!(matches!(
        workflow.replace("session", case, id, revision(), values()),
        Err(ApplicationError::ParticipantRevisionConflict)
    ));
    assert!(matches!(
        workflow.replace("session", case, id, revision(), values()),
        Err(ApplicationError::ParticipantRevisionExhausted)
    ));
    assert!(matches!(
        workflow.change_status("session", case, id, revision(), DirectoryStatus::Archived),
        Err(ApplicationError::StoredParticipantInconsistent(_))
    ));
    assert!(matches!(
        workflow.get("session", case, id),
        Err(ApplicationError::ParticipantNotFound)
    ));
    assert!(matches!(
        workflow.list("session", case, list_query()),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        workflow.history("session", case, id, history_query()),
        Err(ApplicationError::Port(_))
    ));
    assert_eq!(clock.calls(), 7);
}
