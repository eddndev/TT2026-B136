use super::*;
use crate::case_support;
use application::ApplicationError;
use domain::identity::{Role, UserId};
use std::sync::Arc;

mockall::mock! {
    Store {}
    impl AgendaStore for Store {
        fn list(&self, actor: UserId, query: AgendaQuery) -> Result<AgendaPage, ApplicationError>;
    }
}

#[test]
fn precautionary_agenda_reauthenticates_staff_and_denies_clients_before_storage() {
    let q = filtered(AgendaKind::PrecautionaryHearing, HearingStatusFilter::All);
    for role in [Role::Owner, Role::Litigator, Role::Paralegal, Role::Client] {
        let allowed = role != Role::Client;
        let (identity, actor) = case_support::identity(role, if allowed { 2 } else { 1 });
        let mut store = MockStore::new();
        if allowed {
            store
                .expect_list()
                .times(1)
                .withf(move |id, query| *id == actor.id && *query == q)
                .return_once(|_, _| Ok(page(vec![precautionary(from(), Uuid::from_u128(9))])));
        } else {
            store.expect_list().times(0);
        }
        let service = AgendaService::new(Arc::new(store), Arc::new(identity));
        let result = service.list("session", q);
        if allowed {
            assert_eq!(result.unwrap().items.len(), 1);
        } else {
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
        }
    }
}

#[test]
fn precautionary_agenda_does_not_disclose_a_page_after_revocation_or_principal_change() {
    let q = filtered(AgendaKind::PrecautionaryHearing, HearingStatusFilter::All);
    for fault in 0..4 {
        let (_, actor) = case_support::identity(Role::Owner, 0);
        let mut changed = actor.clone();
        match fault {
            0 => changed.id = UserId::new(),
            1 => changed.email = "changed@example.test".into(),
            2 => changed.role = Role::Paralegal,
            _ => (),
        }
        let mut sequence = mockall::Sequence::new();
        let mut identity = case_support::MockIdentity::new();
        let initial = actor.clone();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .withf(|token| token == "session")
            .return_once(move |_| Ok(initial));
        let mut store = MockStore::new();
        store
            .expect_list()
            .times(1)
            .in_sequence(&mut sequence)
            .withf(move |id, query| *id == actor.id && *query == q)
            .return_once(|_, _| Ok(page(vec![precautionary(from(), Uuid::from_u128(9))])));
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .withf(|token| token == "session")
            .return_once(move |_| {
                if fault == 3 {
                    Err(ApplicationError::InvalidSession)
                } else {
                    Ok(changed)
                }
            });
        let service = AgendaService::new(Arc::new(store), Arc::new(identity));
        assert!(matches!(
            service.list("session", q),
            Err(ApplicationError::InvalidSession)
        ));
    }
}
