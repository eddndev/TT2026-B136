use super::*;

#[test]
fn all_staff_roles_can_read_original_decisions_without_being_the_recording_author() {
    let original = operation(10);
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

#[test]
fn client_is_denied_before_any_read_store_method() {
    let original = operation(10);
    let actor = reader(Role::Client);
    for kind in READS {
        let service = service(MockReads::new(), identity(&actor));
        assert!(matches!(
            read(&service, kind, &original),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn invalid_session_is_denied_before_any_read_store_method() {
    let original = operation(10);
    for kind in READS {
        let mut identity = crate::case_support::MockIdentity::new();
        identity
            .expect_authenticate()
            .times(1)
            .return_once(|_| Err(ApplicationError::InvalidSession));
        let service = service(MockReads::new(), identity);
        assert!(matches!(
            read(&service, kind, &original),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

#[test]
fn every_current_principal_field_is_reauthenticated_before_read_disclosure() {
    let original = operation(10);
    for kind in READS {
        for field in 0..3 {
            let actor = reader(Role::Litigator);
            let initial = actor.clone();
            let mut changed = actor.clone();
            match field {
                0 => changed.id = UserId::from_uuid(uuid::Uuid::from_u128(901)),
                1 => changed.email = "new-profile@example.test".into(),
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
                .return_once(move |_| Ok(changed));
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
fn session_revocation_after_store_read_prevents_disclosure() {
    let original = operation(10);
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let initial = actor.clone();
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
            .return_once(|_| Err(ApplicationError::InvalidSession));
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

#[test]
fn current_case_access_denial_is_preserved_without_a_fallback_lookup() {
    let original = operation(10);
    let actor = reader(Role::Owner);
    for kind in READS {
        let mut store = MockReads::new();
        match kind {
            ReadKind::List => {
                store
                    .expect_list()
                    .times(1)
                    .return_once(|_, _, _| Err(ApplicationError::PermissionDenied));
            }
            ReadKind::Get => {
                store
                    .expect_get()
                    .times(1)
                    .return_once(|_, _, _| Err(ApplicationError::PermissionDenied));
            }
            ReadKind::Operation => {
                store
                    .expect_get_operation()
                    .times(1)
                    .return_once(|_, _, _| Err(ApplicationError::PermissionDenied));
            }
        }
        let service = service(store, identity(&actor));
        assert!(matches!(
            read(&service, kind, &original),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}
