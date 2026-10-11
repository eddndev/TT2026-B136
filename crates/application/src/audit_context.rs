//! Request-local audit provenance across synchronous application work.

use crate::ApplicationError;
use domain::{
    audit::{AuditContext, AuditSource},
    identity::UserId,
};
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

thread_local! {
    static ACTIVE: RefCell<Option<AuditScope>> = const { RefCell::new(None) };
}

/// Shared only by work belonging to one request, never by a server worker pool.
#[derive(Clone)]
pub struct AuditScope(Arc<Mutex<AuditContext>>);

impl AuditScope {
    pub fn new(context: AuditContext) -> Self {
        Self(Arc::new(Mutex::new(context)))
    }

    pub fn snapshot(&self) -> Result<AuditContext, ApplicationError> {
        self.0
            .lock()
            .map(|value| value.clone())
            .map_err(|_| ApplicationError::Port("audit context lock poisoned".into()))
    }

    /// The guard restores an outer scope even while unwinding a failed worker.
    pub fn run<T>(&self, operation: impl FnOnce() -> T) -> T {
        struct Restore(Option<AuditScope>);
        impl Drop for Restore {
            fn drop(&mut self) {
                ACTIVE.with(|slot| {
                    slot.replace(self.0.take());
                });
            }
        }
        let _restore = Restore(ACTIVE.with(|slot| slot.replace(Some(self.clone()))));
        operation()
    }
}

pub fn current() -> Option<AuditScope> {
    ACTIVE.with(|slot| slot.borrow().clone())
}

/// Absence of a network request denotes internal work, not a guessed user.
pub fn snapshot() -> Result<AuditContext, ApplicationError> {
    match current() {
        Some(scope) => scope.snapshot(),
        None => Ok(AuditContext {
            user_id: None,
            ip_address: None,
            source: AuditSource::Internal,
            request_id: None,
        }),
    }
}

/// Called only after current session and account admission have both succeeded.
pub fn bind_actor(user: UserId) -> Result<(), ApplicationError> {
    if let Some(scope) = current() {
        let mut context = scope
            .0
            .lock()
            .map_err(|_| ApplicationError::Port("audit context lock poisoned".into()))?;
        if context.user_id.is_some_and(|previous| previous != user) {
            return Err(ApplicationError::InvalidSession);
        }
        context.user_id = Some(user);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{audit::AuditSource, identity::UserId};

    fn http() -> AuditContext {
        AuditContext {
            user_id: None,
            ip_address: Some("192.0.2.4".parse().unwrap()),
            source: AuditSource::Http,
            request_id: Some(uuid::Uuid::new_v4()),
        }
    }

    #[test]
    fn verified_identity_is_shared_between_workers_but_not_other_requests() {
        let scope = AuditScope::new(http());
        let user = UserId::new();
        scope.run(|| bind_actor(user).unwrap());
        let next = scope.clone();
        assert_eq!(
            std::thread::spawn(move || next.run(snapshot))
                .join()
                .unwrap()
                .unwrap()
                .user_id,
            Some(user)
        );
        assert_eq!(snapshot().unwrap().source, AuditSource::Internal);
        assert_eq!(AuditScope::new(http()).snapshot().unwrap().user_id, None);
    }

    #[test]
    fn nested_scopes_and_panics_restore_the_previous_context() {
        let outer = AuditScope::new(http());
        let inner = AuditScope::new(http());
        outer.run(|| {
            let original = snapshot().unwrap();
            let result = std::panic::catch_unwind(|| inner.run(|| panic!("test panic")));
            assert!(result.is_err());
            assert_eq!(snapshot().unwrap(), original);
        });
        assert!(current().is_none());
    }

    #[test]
    fn another_principal_cannot_replace_the_verified_request_identity() {
        let scope = AuditScope::new(http());
        scope.run(|| {
            let user = UserId::new();
            bind_actor(user).unwrap();
            bind_actor(user).unwrap();
            assert!(bind_actor(UserId::new()).is_err());
            assert_eq!(snapshot().unwrap().user_id, Some(user));
        });
    }
}
