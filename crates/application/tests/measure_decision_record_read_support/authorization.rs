use super::*;

#[test]
fn all_current_staff_roles_can_read_records_without_being_the_historical_author() {
    let original = v2(10);
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let actor = reader(role);
        for kind in READS {
            let store = successful_store(&actor, &original, original.clone(), kind);
            assert_eq!(
                read(&service(store, identity(&actor)), kind, &original).unwrap(),
                vec![original.clone()]
            );
        }
    }
}

#[test]
fn clients_and_invalid_sessions_cannot_reach_any_record_read_store_method() {
    let original = v1(10);
    for kind in READS {
        assert!(matches!(
            read(
                &service(MockReads::new(), identity(&reader(Role::Client))),
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
fn every_current_principal_field_and_revocation_are_rechecked_before_disclosure() {
    let original = v1(10);
    for kind in READS {
        for field in 0..4 {
            let actor = reader(Role::Litigator);
            let initial = actor.clone();
            let mut changed = actor.clone();
            match field {
                0 => changed.id = UserId::from_uuid(Uuid::from_u128(901)),
                1 => changed.email = "different@example.test".into(),
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
                    if field == 3 {
                        Err(ApplicationError::InvalidSession)
                    } else {
                        Ok(changed)
                    }
                });
            let store = successful_store(&actor, &original, original.clone(), kind);
            assert!(matches!(
                read(&service(store, identity), kind, &original),
                Err(ApplicationError::InvalidSession)
            ));
        }
    }
}

#[test]
fn case_denial_absence_and_audit_failure_remain_errors_without_fallback() {
    let original = v1(10);
    let actor = reader(Role::Owner);
    for kind in READS {
        for variant in 0..3 {
            let error = match variant {
                0 => ApplicationError::PermissionDenied,
                1 => MeasureDecisionError::NotFound.into(),
                _ => ApplicationError::Port("measure record access audit failed".into()),
            };
            let mut store = MockReads::new();
            match kind {
                ReadKind::List => {
                    store
                        .expect_list()
                        .times(1)
                        .return_once(move |_, _, _| Err(error));
                }
                ReadKind::Get => {
                    store
                        .expect_get()
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
                    Err(ApplicationError::MeasureDecision(
                        MeasureDecisionError::NotFound
                    ))
                )),
                _ => assert!(matches!(result, Err(ApplicationError::Port(_)))),
            }
        }
    }
}
