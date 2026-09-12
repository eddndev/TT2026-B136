use std::sync::Arc;

use application::identity::{IdentityService, UserRecord, UserRepository};
use application::ApplicationError;
use domain::crypto::{PasswordHasher, PasswordVerification, RecoveryCodeSet};
use domain::identity::UserId;
use domain::DomainError;

use super::{ports, MemorySessions, MemoryUsers};

/// Allows authentication checks while rejecting side effects of invalid email.
pub fn guarded_service(users: Arc<MemoryUsers>, sessions: Arc<MemorySessions>) -> IdentityService {
    let mut ports = ports(users.clone(), sessions);
    ports.users = Arc::new(GuardedUsers(users));
    ports.passwords = Arc::new(ForbiddenPasswords);
    IdentityService::new(ports)
}

struct GuardedUsers(Arc<MemoryUsers>);

impl UserRepository for GuardedUsers {
    fn has_users(&self) -> Result<bool, ApplicationError> {
        self.0.has_users()
    }

    fn find_by_id(&self, id: UserId) -> Result<Option<UserRecord>, ApplicationError> {
        self.0.find_by_id(id)
    }

    fn find_by_email(&self, _: &str) -> Result<Option<UserRecord>, ApplicationError> {
        panic!("invalid email must not reach a repository lookup")
    }

    fn insert_initial_owner(&self, _: UserRecord) -> Result<bool, ApplicationError> {
        panic!("invalid email must not create the initial owner")
    }

    fn insert(&self, _: UserRecord) -> Result<(), ApplicationError> {
        panic!("invalid email must not create a user")
    }

    fn replace_recovery_codes(
        &self,
        _: UserId,
        _: u64,
        _: RecoveryCodeSet,
    ) -> Result<(), ApplicationError> {
        panic!("invalid email must not modify recovery codes")
    }
}

struct ForbiddenPasswords;

impl PasswordHasher for ForbiddenPasswords {
    fn hash(&self, _: &str) -> Result<String, DomainError> {
        panic!("invalid email must not trigger password hashing")
    }

    fn verify(&self, _: &str, _: &str) -> Result<PasswordVerification, DomainError> {
        panic!("invalid email must not trigger password verification")
    }
}
