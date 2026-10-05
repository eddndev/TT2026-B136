use super::*;

#[test]
fn every_staff_role_can_read_a_consistent_closed_context() {
    use application::cases::case_administration_digest;
    use domain::case_administration::CaseAdministrativeStatus;
    let mut material = crate::context_support::initial();
    crate::context_support::observed_newer(&mut material);
    material.administration.values = material
        .administration
        .values
        .with_status(CaseAdministrativeStatus::Closed);
    material.administration.values_digest =
        case_administration_digest(&Hasher, &material.administration.values);
    let context = PrecautionaryContext::new(&Hasher, material).unwrap();
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let actor = reader(role);
        let case = context.material().case_id;
        let store = returning(&actor, case, context.clone());
        assert_eq!(
            service(store, identity(&actor), clock())
                .get("session", case)
                .unwrap(),
            context
        );
    }
}

#[test]
fn client_and_invalid_session_are_denied_before_context_lookup() {
    let case = context().material().case_id;
    assert!(matches!(
        service(MockReads::new(), identity(&reader(Role::Client)), clock()).get("session", case),
        Err(ApplicationError::PermissionDenied)
    ));
    let mut identity = crate::case_support::MockIdentity::new();
    identity
        .expect_authenticate()
        .times(1)
        .return_once(|_| Err(ApplicationError::InvalidSession));
    assert!(matches!(
        service(MockReads::new(), identity, clock()).get("session", case),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn changed_full_current_principal_or_revocation_prevents_disclosure() {
    let actor = reader(Role::Litigator);
    let context = context();
    let case = context.material().case_id;
    for mutation in 0..4 {
        let before = actor.clone();
        let mut after = actor.clone();
        match mutation {
            0 => after.id = UserId::new(),
            1 => after.email = "changed-reader@example.test".into(),
            2 => after.role = Role::Owner,
            _ => {}
        }
        let mut identity = crate::case_support::MockIdentity::new();
        let mut sequence = mockall::Sequence::new();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(before));
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                if mutation == 3 {
                    Err(ApplicationError::InvalidSession)
                } else {
                    Ok(after)
                }
            });
        let store = returning(&actor, case, context.clone());
        assert!(matches!(
            service(store, identity, clock()).get("session", case),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

#[test]
fn access_absence_and_transactional_read_audit_errors_are_preserved() {
    let actor = reader(Role::Owner);
    let case = context().material().case_id;
    for error in [
        ApplicationError::PermissionDenied,
        ApplicationError::CaseNotFound,
        ApplicationError::Port("context read audit rejected".into()),
    ] {
        let message = error.to_string();
        let mut store = MockReads::new();
        store
            .expect_get()
            .times(1)
            .return_once(move |_, _| Err(error));
        assert_eq!(
            service(store, identity(&actor), clock())
                .get("session", case)
                .unwrap_err()
                .to_string(),
            message
        );
    }
}
