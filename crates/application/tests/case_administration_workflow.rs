mod case_administration_support;
#[allow(dead_code)]
mod case_support;

use application::cases::*;
use application::ApplicationError;
use case_administration_support::{creation, detail, expected, history_query, list_query, values};
use case_support::{identity, instant, service, MockCases, MockIdentity};
use domain::cases::{CaseId, CaseMetadata};
use domain::identity::{Role, UserId};

#[test]
fn clients_and_revoked_sessions_reach_neither_staff_repository_nor_clock() {
    for revoked in [false, true] {
        let auth = if revoked {
            let mut mock = MockIdentity::new();
            mock.expect_authenticate()
                .times(6)
                .returning(|_| Err(ApplicationError::InvalidSession));
            mock
        } else {
            identity(Role::Client, 6).0
        };
        let (workflow, clock) = service(MockCases::new(), auth);
        let id = CaseId::new();
        for error in [
            workflow.register_penal("session", creation()).unwrap_err(),
            workflow
                .replace_administration("session", id, expected(), values())
                .unwrap_err(),
            workflow
                .change_administrative_status(
                    "session",
                    id,
                    expected(),
                    CaseAdministrativeStatus::Closed,
                )
                .unwrap_err(),
            workflow
                .list_administrations("session", list_query())
                .unwrap_err(),
            workflow.get_administration("session", id).unwrap_err(),
            workflow
                .administration_history("session", id, history_query())
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
fn paralegal_cannot_register_replace_or_change_administrative_status() {
    let (workflow, clock) = service(MockCases::new(), identity(Role::Paralegal, 3).0);
    let id = CaseId::new();
    for error in [
        workflow.register_penal("session", creation()).unwrap_err(),
        workflow
            .replace_administration("session", id, expected(), values())
            .unwrap_err(),
        workflow
            .change_administrative_status(
                "session",
                id,
                expected(),
                CaseAdministrativeStatus::Active,
            )
            .unwrap_err(),
    ] {
        assert!(matches!(error, ApplicationError::PermissionDenied));
    }
    assert_eq!(clock.calls(), 0);
}

#[test]
fn penal_registration_generates_identity_and_returns_only_the_commit_projection() {
    for role in [Role::Owner, Role::Litigator] {
        let (auth, actor) = identity(role, 1);
        let user = actor.id;
        let mut repository = MockCases::new();
        repository
            .expect_register_penal()
            .times(1)
            .withf(move |who, id, input, at| {
                *who == user
                    && id.as_uuid().get_version_num() == 4
                    && *input == creation()
                    && *at == instant()
            })
            .return_once(move |_, id, _, _| Ok(detail(id, user, 1)));
        let (workflow, clock) = service(repository, auth);
        let result = workflow.register_penal("session", creation()).unwrap();
        assert_eq!(result, detail(result.origin.id, user, 1));
        assert_ne!(
            result.administration.snapshot().unwrap().changed_by.email,
            actor.email
        );
        assert_eq!(clock.calls(), 1);
    }
}

#[test]
fn replace_forwards_zero_or_positive_expectation_without_reading_a_head() {
    for head in [CaseRevisionExpectation::Unrevised, expected()] {
        for role in [Role::Owner, Role::Litigator] {
            let (auth, actor) = identity(role, 1);
            let user = actor.id;
            let id = CaseId::new();
            let mut returned = detail(id, user, head.get() + 1);
            returned.initial_stage = None;
            let expected_result = returned.clone();
            let mut repository = MockCases::new();
            repository
                .expect_replace_administration()
                .times(1)
                .withf(move |who, scope, revision, input, at| {
                    *who == user
                        && *scope == id
                        && *revision == head
                        && *input == values()
                        && *at == instant()
                })
                .return_once(move |_, _, _, _, _| Ok(returned));
            let (workflow, clock) = service(repository, auth);
            assert_eq!(
                workflow
                    .replace_administration("session", id, head, values())
                    .unwrap(),
                expected_result
            );
            assert_eq!(clock.calls(), 1);
        }
    }
}

#[test]
fn status_only_preserves_the_repository_current_text_and_original_stage() {
    let (auth, actor) = identity(Role::Litigator, 1);
    let user = actor.id;
    let id = CaseId::new();
    let mut result = detail(id, user, 8);
    if let CurrentCaseAdministration::Recorded(snapshot) = &mut result.administration {
        snapshot.values = CaseAdministrationValues::new(
            CaseEditableValues::new(
                CaseMetadata::new("Concurrent title", "New reference").unwrap(),
                Some(creation().profile().clone()),
            ),
            CaseAdministrativeStatus::Closed,
        );
    }
    let returned = result.clone();
    let mut repository = MockCases::new();
    repository
        .expect_change_administrative_status()
        .times(1)
        .withf(move |who, scope, revision, status, at| {
            *who == user
                && *scope == id
                && *revision == expected()
                && *status == CaseAdministrativeStatus::Closed
                && *at == instant()
        })
        .return_once(move |_, _, _, _, _| Ok(returned));
    let (workflow, clock) = service(repository, auth);
    assert_eq!(
        workflow
            .change_administrative_status(
                "session",
                id,
                expected(),
                CaseAdministrativeStatus::Closed
            )
            .unwrap(),
        result
    );
    assert_eq!(clock.calls(), 1);
}

#[test]
fn staff_reads_forward_actor_scope_filters_history_and_repository_provenance() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let (auth, actor) = identity(role, 3);
        let user = actor.id;
        let id = CaseId::new();
        let result = detail(id, UserId::new(), 7);
        let page = CaseAdministrationPage {
            cases: vec![CaseAdministrationOverview {
                origin: result.origin.clone(),
                metadata: result.administration.values().metadata().clone(),
                revision: Some(CaseRevision::new(7).unwrap()),
                administrative_status: CaseAdministrativeStatus::Closed,
                penal_identifiers: Some(CasePenalIdentifiers {
                    nuc: "NUC-1".into(),
                    judicial_case_number: "CJ-1".into(),
                }),
                initial_stage: Some(InitialCaseStage::Investigation),
            }],
            has_more: true,
            next_after_id: Some(id),
        };
        let history = CaseAdministrationHistoryPage {
            revisions: vec![result.administration.snapshot().unwrap().clone()],
            has_more: true,
            next_before_revision: Some(CaseRevision::new(7).unwrap()),
        };
        let returned = result.clone();
        let returned_page = page.clone();
        let returned_history = history.clone();
        let mut repository = MockCases::new();
        repository
            .expect_get_administration()
            .times(1)
            .withf(move |who, scope, at| *who == user && *scope == id && *at == instant())
            .return_once(move |_, _, _| Ok(returned));
        repository
            .expect_list_administrations()
            .times(1)
            .withf(move |who, query, at| *who == user && *query == list_query() && *at == instant())
            .return_once(move |_, _, _| Ok(returned_page));
        repository
            .expect_administration_history()
            .times(1)
            .withf(move |who, scope, query, at| {
                *who == user && *scope == id && *query == history_query() && *at == instant()
            })
            .return_once(move |_, _, _, _| Ok(returned_history));
        let (workflow, clock) = service(repository, auth);
        assert_eq!(workflow.get_administration("session", id).unwrap(), result);
        assert_eq!(
            workflow
                .list_administrations("session", list_query())
                .unwrap(),
            page
        );
        assert_eq!(
            workflow
                .administration_history("session", id, history_query())
                .unwrap(),
            history
        );
        assert_eq!(clock.calls(), 3);
    }
}

#[test]
fn commit_errors_and_closed_or_conflicting_heads_propagate_without_followup_gets() {
    let (auth, _) = identity(Role::Owner, 9);
    let mut repository = MockCases::new();
    repository
        .expect_register_penal()
        .times(1)
        .returning(|_, _, _, _| Err(ApplicationError::CaseIdentifierConflict));
    let mut sequence = mockall::Sequence::new();
    for error in [
        ApplicationError::CaseRevisionConflict,
        ApplicationError::CaseRevisionExhausted,
        ApplicationError::CaseClosed,
        ApplicationError::CaseProfileRequired,
    ] {
        repository
            .expect_replace_administration()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_, _, _, _, _| Err(error));
    }
    repository
        .expect_change_administrative_status()
        .times(1)
        .returning(|_, _, _, _, _| {
            Err(ApplicationError::StoredCaseAdministrationInconsistent(
                "digest".into(),
            ))
        });
    repository
        .expect_get_administration()
        .times(1)
        .returning(|_, _, _| Err(ApplicationError::CaseNotFound));
    repository
        .expect_list_administrations()
        .times(1)
        .returning(|_, _, _| Err(ApplicationError::Port("audit failed".into())));
    repository
        .expect_administration_history()
        .times(1)
        .returning(|_, _, _, _| Err(ApplicationError::Port("commit failed".into())));
    let (workflow, clock) = service(repository, auth);
    let id = CaseId::new();
    assert!(matches!(
        workflow.register_penal("session", creation()),
        Err(ApplicationError::CaseIdentifierConflict)
    ));
    let errors: Vec<_> = (0..4)
        .map(|_| {
            workflow
                .replace_administration("session", id, expected(), values())
                .unwrap_err()
        })
        .collect();
    assert!(matches!(errors[0], ApplicationError::CaseRevisionConflict));
    assert!(matches!(errors[1], ApplicationError::CaseRevisionExhausted));
    assert!(matches!(errors[2], ApplicationError::CaseClosed));
    assert!(matches!(errors[3], ApplicationError::CaseProfileRequired));
    assert!(matches!(
        workflow.change_administrative_status(
            "session",
            id,
            expected(),
            CaseAdministrativeStatus::Active
        ),
        Err(ApplicationError::StoredCaseAdministrationInconsistent(_))
    ));
    assert!(matches!(
        workflow.get_administration("session", id),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        workflow.list_administrations("session", list_query()),
        Err(ApplicationError::Port(_))
    ));
    assert!(matches!(
        workflow.administration_history("session", id, history_query()),
        Err(ApplicationError::Port(_))
    ));
    assert_eq!(clock.calls(), 9);
}

#[test]
fn role_is_reauthenticated_before_each_administration_mutation() {
    let id = UserId::new();
    let mut auth = MockIdentity::new();
    let mut sequence = mockall::Sequence::new();
    for role in [Role::Owner, Role::Paralegal] {
        auth.expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                Ok(application::identity::Principal {
                    id,
                    email: "actor@example.com".into(),
                    role,
                })
            });
    }
    let mut repository = MockCases::new();
    repository
        .expect_change_administrative_status()
        .times(1)
        .return_once(move |_, id, _, _, _| Ok(detail(id, UserId::new(), 8)));
    let (workflow, clock) = service(repository, auth);
    let case = CaseId::new();
    workflow
        .change_administrative_status(
            "session",
            case,
            expected(),
            CaseAdministrativeStatus::Closed,
        )
        .unwrap();
    assert!(matches!(
        workflow.change_administrative_status(
            "session",
            case,
            expected(),
            CaseAdministrativeStatus::Active
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(clock.calls(), 1);
}
