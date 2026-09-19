#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod deadline_support;
#[allow(dead_code)]
mod hearing_support;
#[allow(dead_code)]
mod procedural_fact_service_support;
mod procedural_resource_support;
mod resource_activity_support;
use application::{resource_activities::*, ApplicationError};
use domain::{cases::CaseId, crypto::Sha256Digest, identity::Role};
use procedural_resource_support::identity_for;
use resource_activity_support::*;

#[test]
fn client_and_paralegal_cannot_mutate_and_no_target_is_looked_up_first() {
    let (_, owner) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&owner);
    for role in [Role::Client, Role::Paralegal] {
        let (identity, _) = case_support::identity(role, 1);
        assert!(matches!(
            service(MockStore::new(), identity).prepare(
                "session",
                fixture.case_id,
                fixture.resource_id,
                fixture.command.clone(),
            ),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn closed_material_and_stale_expected_resource_head_are_rejected_in_application() {
    use application::cases::*;
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    for closed in [true, false] {
        let mut material = fixture.material.clone();
        let mut command = fixture.command.clone();
        if closed {
            let values = material
                .administration
                .values()
                .with_status(CaseAdministrativeStatus::Closed);
            material.administration =
                CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
                    case_id: fixture.case_id,
                    revision: CaseRevision::FIRST,
                    values_digest: case_administration_digest(
                        hearing_support::hasher().as_ref(),
                        &values,
                    ),
                    values,
                    changed_at: case_support::instant(),
                    changed_by: fixture.material.resource_head.recorded_by.clone(),
                }));
        } else {
            command.expected_resource_revision =
                domain::procedural_resources::ResourceRevision::initial();
        }
        let (identity, _) = identity_for(actor.clone(), 1);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| {
                Ok(ResourceActivityPreparation::Ready(Box::new(material)))
            });
        let error = service(store, identity)
            .prepare("session", fixture.case_id, fixture.resource_id, command)
            .unwrap_err();
        assert!(matches!(
            (closed, error),
            (true, ApplicationError::CaseClosed)
                | (
                    false,
                    ApplicationError::ResourceActivity(
                        ResourceActivityError::ResourceRevisionConflict
                    )
                )
        ));
    }
}

#[test]
fn revoked_account_after_prepare_cannot_return_capture_or_reach_commit() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    let expected = fixture.draft(&actor).submission_digest;
    let mut identity = case_support::MockIdentity::new();
    let mut calls = 0;
    identity.expect_authenticate().times(2).returning(move |_| {
        calls += 1;
        if calls == 1 {
            Ok(actor.clone())
        } else {
            Err(ApplicationError::InvalidSession)
        }
    });
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| {
            Ok(ResourceActivityPreparation::Ready(Box::new(
                fixture.material,
            )))
        });
    assert!(matches!(
        service(store, identity).submit(
            "session",
            fixture.case_id,
            fixture.resource_id,
            fixture.command,
            expected,
        ),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn preparation_denials_and_closure_survive_without_a_partial_commit() {
    for reason in [0, 1, 2] {
        let (identity, actor) = case_support::identity(Role::Litigator, 1);
        let fixture = Fixture::new(&actor);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| {
                Err(match reason {
                    0 => ApplicationError::PermissionDenied,
                    1 => ApplicationError::CaseClosed,
                    _ => ResourceActivityError::ResourceRevisionConflict.into(),
                })
            });
        let error = service(store, identity)
            .prepare(
                "session",
                fixture.case_id,
                fixture.resource_id,
                fixture.command,
            )
            .unwrap_err();
        assert!(matches!(
            (reason, error),
            (0, ApplicationError::PermissionDenied)
                | (1, ApplicationError::CaseClosed)
                | (
                    2,
                    ApplicationError::ResourceActivity(
                        ResourceActivityError::ResourceRevisionConflict
                    )
                )
        ));
    }
}

#[test]
fn foreign_scopes_exact_revision_substitution_and_corrupt_captures_fail_before_commit() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    for field in 0..6 {
        let mut material = fixture.material.clone();
        match field {
            0 => material.case_id = CaseId::new(),
            1 => material.sources.resource.case_id = CaseId::new(),
            2 => material.sources.resource = material.resource_head.clone(),
            3 => {
                material
                    .sources
                    .act
                    .as_mut()
                    .unwrap()
                    .receipt
                    .capture_digest = Sha256Digest::from_array([9; 32])
            }
            4 => {
                let ResourceActivityTargetDetail::Hearing(value) = &mut material.sources.target
                else {
                    unreachable!()
                };
                value.snapshot.case_id = CaseId::new();
            }
            _ => {
                let ResourceActivityTargetDetail::Hearing(value) = &mut material.sources.target
                else {
                    unreachable!()
                };
                value.snapshot.receipt.submission_digest = Sha256Digest::from_array([9; 32]);
            }
        }
        let (identity, _) = identity_for(actor.clone(), 1);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| {
                Ok(ResourceActivityPreparation::Ready(Box::new(material)))
            });
        assert!(
            service(store, identity)
                .prepare(
                    "session",
                    fixture.case_id,
                    fixture.resource_id,
                    fixture.command.clone(),
                )
                .is_err(),
            "invalid material field {field}"
        );
    }
}

#[test]
fn membership_or_head_changed_under_commit_lock_is_not_retried() {
    let (_, actor) = case_support::identity(Role::Litigator, 0);
    let fixture = Fixture::new(&actor);
    let expected = fixture.draft(&actor).submission_digest;
    for revoked in [true, false] {
        let (identity, _) = identity_for(actor.clone(), 2);
        let mut store = MockStore::new();
        let material = fixture.material.clone();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| {
                Ok(ResourceActivityPreparation::Ready(Box::new(material)))
            });
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, _, _| {
                Err(if revoked {
                    ApplicationError::PermissionDenied
                } else {
                    ResourceActivityError::RevisionConflict.into()
                })
            });
        let error = service(store, identity)
            .submit(
                "session",
                fixture.case_id,
                fixture.resource_id,
                fixture.command.clone(),
                expected,
            )
            .unwrap_err();
        assert!(matches!(
            (revoked, error),
            (true, ApplicationError::PermissionDenied)
                | (
                    false,
                    ApplicationError::ResourceActivity(ResourceActivityError::RevisionConflict)
                )
        ));
    }
}

#[test]
fn paralegal_reads_exact_link_history_and_client_is_denied_before_lookup() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let mut fixture = Fixture::new(&actor);
    let linked = fixture.committed(&actor);
    fixture.unlink(linked.clone());
    let unlinked = fixture.committed(&actor);
    let (identity, _) = case_support::identity(Role::Paralegal, 4);
    let mut store = MockStore::new();
    let exact = linked.clone();
    store
        .expect_get()
        .times(1)
        .withf(move |_, _, _, _, revision, _| {
            *revision == Some(ResourceActivityRevision::initial())
        })
        .return_once(move |_, _, _, _, _, _| Ok(hearing_view(exact)));
    let rows = vec![unlinked, linked.clone()];
    store
        .expect_history()
        .times(1)
        .return_once(move |_, _, _, _, _, _| {
            Ok(ResourceActivityHistoryPage {
                revisions: rows,
                has_more: false,
                next_before_revision: None,
            })
        });
    let workflow = service(store, identity);
    assert_eq!(
        workflow
            .get(
                "session",
                fixture.case_id,
                fixture.resource_id,
                linked.id,
                Some(linked.revision),
            )
            .unwrap()
            .association,
        linked
    );
    assert_eq!(
        workflow
            .history(
                "session",
                fixture.case_id,
                fixture.resource_id,
                linked.id,
                ResourceActivityHistoryQuery::new(20, None).unwrap(),
            )
            .unwrap()
            .revisions[1],
        linked
    );
    let (identity, _) = case_support::identity(Role::Client, 1);
    assert!(matches!(
        service(MockStore::new(), identity).get(
            "session",
            fixture.case_id,
            fixture.resource_id,
            linked.id,
            None,
        ),
        Err(ApplicationError::PermissionDenied)
    ));
}

#[test]
fn exact_association_read_retains_old_hearing_and_exposes_real_head_separately() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    let linked = fixture.committed(&actor);
    let ResourceActivityTargetDetail::Hearing(original) = &linked.sources.target else {
        unreachable!()
    };
    let replacement = hearing_support::replacement(original);
    let head = hearing_support::detail(
        fixture.case_id,
        actor.id,
        &replacement,
        original.snapshot.values.clone(),
        &hearing_support::context(fixture.case_id, actor.id),
    );
    let view = ResourceActivityView {
        association: linked.clone(),
        checked_at: case_support::instant(),
        current_target: ResourceActivityCurrentTarget::Hearing(Box::new(head.clone())),
    };
    let (identity, _) = identity_for(actor, 2);
    let mut store = MockStore::new();
    store
        .expect_get()
        .times(1)
        .return_once(move |_, _, _, _, _, _| Ok(view));
    let result = service(store, identity)
        .get(
            "session",
            fixture.case_id,
            fixture.resource_id,
            linked.id,
            Some(linked.revision),
        )
        .unwrap();
    assert_eq!(result.association.sources.target, linked.sources.target);
    assert_eq!(
        result.current_target,
        ResourceActivityCurrentTarget::Hearing(Box::new(head))
    );
}
