use super::*;

#[test]
fn all_staff_roles_read_under_the_full_current_principal() {
    let saved = operation(40);
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let actor = reader(role);
        for kind in READS {
            let store = successful_store(&actor, &saved, saved.clone(), kind);
            assert_eq!(
                read(&service(store, identity(&actor), clock()), &saved, kind).unwrap(),
                vec![saved.clone()]
            );
        }
    }
}

#[test]
fn clients_and_invalid_sessions_are_denied_before_any_read_port_call() {
    let saved = operation(40);
    for kind in READS {
        assert!(matches!(
            read(
                &service(MockReads::new(), identity(&reader(Role::Client)), clock()),
                &saved,
                kind
            ),
            Err(ApplicationError::PermissionDenied)
        ));
        let mut identity = crate::case_support::MockIdentity::new();
        identity
            .expect_authenticate()
            .times(1)
            .return_once(|_| Err(ApplicationError::InvalidSession));
        assert!(matches!(
            read(&service(MockReads::new(), identity, clock()), &saved, kind),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

#[test]
fn changes_to_any_current_principal_field_or_revocation_prevent_disclosure() {
    let saved = operation(40);
    let actor = reader(Role::Litigator);
    for kind in READS {
        for change in 0..4 {
            let first = actor.clone();
            let mut second = actor.clone();
            match change {
                0 => second.id = UserId::new(),
                1 => second.email = "other@example.test".into(),
                2 => second.role = Role::Owner,
                _ => {}
            }
            let mut identity = crate::case_support::MockIdentity::new();
            let mut sequence = mockall::Sequence::new();
            identity
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| Ok(first));
            identity
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| {
                    if change == 3 {
                        Err(ApplicationError::InvalidSession)
                    } else {
                        Ok(second)
                    }
                });
            let store = successful_store(&actor, &saved, saved.clone(), kind);
            assert!(matches!(
                read(&service(store, identity, clock()), &saved, kind),
                Err(ApplicationError::InvalidSession)
            ));
        }
    }
}

#[test]
fn access_absence_and_read_audit_errors_are_returned_without_partial_results() {
    let saved = operation(40);
    let actor = reader(Role::Owner);
    for kind in READS {
        for error in [
            ApplicationError::CaseNotFound,
            ApplicationError::PermissionDenied,
            PrecautionaryHearingError::NotFound.into(),
            ApplicationError::Port("read audit failed".into()),
        ] {
            let expected = error.to_string();
            let mut store = MockReads::new();
            match kind {
                ReadKind::List => {
                    store
                        .expect_list()
                        .times(1)
                        .return_once(move |_, _, _| Err(error));
                }
                ReadKind::Current | ReadKind::Exact => {
                    store
                        .expect_get()
                        .times(1)
                        .return_once(move |_, _, _, _| Err(error));
                }
                ReadKind::Operation => {
                    store
                        .expect_get_operation()
                        .times(1)
                        .return_once(move |_, _, _| Err(error));
                }
            }
            assert_eq!(
                read(&service(store, identity(&actor), clock()), &saved, kind)
                    .unwrap_err()
                    .to_string(),
                expected
            );
        }
    }
}
