use super::*;

#[test]
fn all_staff_roles_read_original_corrections_and_marks_without_being_the_author() {
    let original = operation(10);
    let mut marked = from_group(
        &root_fixture(20).capture(),
        &crate::effect_support::empty_history(),
        20,
    );
    marked.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    for original in [original, stored(&marked)] {
        for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
            let actor = reader(role);
            for kind in READS {
                let service = service(
                    successful_store(&actor, &original, original.clone(), kind),
                    identity(&actor),
                );
                assert_eq!(
                    read(&service, kind, &original).unwrap(),
                    vec![original.clone()]
                );
            }
        }
    }
}

#[test]
fn clients_and_invalid_sessions_are_denied_before_any_store_read() {
    let original = operation(10);
    for kind in READS {
        let actor = reader(Role::Client);
        assert!(matches!(
            read(
                &service(MockReads::new(), identity(&actor)),
                kind,
                &original
            ),
            Err(ApplicationError::PermissionDenied)
        ));
        let mut identity = crate::case_support::MockIdentity::new();
        identity
            .expect_authenticate()
            .times(1)
            .return_once(|_| Err(ApplicationError::InvalidSession));
        assert!(matches!(
            read(&service(MockReads::new(), identity), kind, &original),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

#[test]
fn every_principal_field_and_session_are_rechecked_before_disclosure() {
    let original = operation(10);
    for kind in READS {
        for mutation in 0..4 {
            let actor = reader(Role::Litigator);
            let initial = actor.clone();
            let mut changed = actor.clone();
            match mutation {
                0 => changed.id = UserId::from_uuid(Uuid::from_u128(901)),
                1 => changed.email = "changed@example.test".into(),
                _ => changed.role = Role::Paralegal,
            }
            let mut identity = crate::case_support::MockIdentity::new();
            let mut sequence = mockall::Sequence::new();
            identity
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| Ok(initial));
            identity
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| {
                    if mutation == 3 {
                        Err(ApplicationError::InvalidSession)
                    } else {
                        Ok(changed)
                    }
                });
            let service = service(
                successful_store(&actor, &original, original.clone(), kind),
                identity,
            );
            assert!(matches!(
                read(&service, kind, &original),
                Err(ApplicationError::InvalidSession)
            ));
        }
    }
}

#[test]
fn case_denial_not_found_and_failed_audit_are_propagated_without_disclosure_or_fallback() {
    let original = operation(10);
    let actor = reader(Role::Owner);
    for kind in READS {
        for variant in 0..3 {
            let error = match variant {
                0 => ApplicationError::PermissionDenied,
                1 => MeasureAdministrativeError::NotFound.into(),
                _ => ApplicationError::Port("administrative read audit append failed".into()),
            };
            let mut store = MockReads::new();
            match kind {
                ReadKind::List => {
                    store
                        .expect_list()
                        .times(1)
                        .return_once(move |_, _, _| Err(error));
                }
                ReadKind::Operation => {
                    store
                        .expect_get_operation()
                        .times(1)
                        .return_once(move |_, _, _| Err(error));
                }
            }
            let result = read(&service(store, identity(&actor)), kind, &original);
            match variant {
                0 => assert!(matches!(result, Err(ApplicationError::PermissionDenied))),
                1 => assert!(matches!(
                    result,
                    Err(ApplicationError::MeasureAdministrative(
                        MeasureAdministrativeError::NotFound
                    ))
                )),
                _ => assert!(matches!(result, Err(ApplicationError::Port(_)))),
            }
        }
    }
}
