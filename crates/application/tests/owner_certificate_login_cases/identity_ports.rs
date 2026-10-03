use application::identity::{SecretProtector, UserRepository};
use domain::{
    audit::{AuditEvent, AuditLog, ChainedEvent},
    clock::{Clock, OffsetDateTime},
    crypto::{
        PasswordHasher, PasswordVerification, RecoveryCodeGenerator, RecoveryCodeSet, Sha256Digest,
        TotpEnrollment, TotpProvider, TotpVerification,
    },
    identity::UserId,
    DomainError,
};
use zeroize::Zeroizing;

use super::{at, ApplicationError, Env, UserRecord};

impl UserRepository for Env {
    fn has_users(&self) -> Result<bool, ApplicationError> {
        Ok(true)
    }

    fn insert_initial_owner(
        &self,
        _: UserRecord,
        _: OffsetDateTime,
    ) -> Result<bool, ApplicationError> {
        panic!("login cannot enroll an owner")
    }

    fn insert(&self, _: UserRecord, _: UserId, _: OffsetDateTime) -> Result<(), ApplicationError> {
        panic!("login cannot create a user")
    }

    fn find_by_email(&self, email: &str) -> Result<Option<UserRecord>, ApplicationError> {
        Ok(self.read(|s| (s.user.email == email).then(|| s.user.clone())))
    }

    fn find_by_id(&self, id: UserId) -> Result<Option<UserRecord>, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        let user = (state.user.id == id).then(|| state.user.clone());
        if let Some(change) = state.after_user_find.take() {
            state.change(change);
        }
        Ok(user)
    }

    fn replace_recovery_codes(
        &self,
        id: UserId,
        revision: u64,
        codes: RecoveryCodeSet,
        _: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        let mut state = self.0.lock().unwrap();
        if state.user.id != id || state.user.revision != revision {
            return Err(ApplicationError::ConcurrentModification);
        }
        state.user.recovery_codes = codes;
        state.user.revision += 1;
        let revision = state.user.revision;
        state.context.as_mut().unwrap().account.revision = revision;
        if let Some(change) = state.after_factor.take() {
            state.change(change);
        }
        Ok(())
    }
}

impl Clock for Env {
    fn now(&self) -> OffsetDateTime {
        at(self.read(|s| s.now))
    }
}

impl PasswordHasher for Env {
    fn hash(&self, value: &str) -> Result<String, DomainError> {
        Ok(format!("fake:{value}"))
    }

    fn verify(&self, value: &str, stored: &str) -> Result<PasswordVerification, DomainError> {
        Ok(if stored == format!("fake:{value}") {
            PasswordVerification::Match
        } else {
            PasswordVerification::Mismatch
        })
    }
}

impl TotpProvider for Env {
    fn enroll(&self, _: &str) -> Result<TotpEnrollment, DomainError> {
        panic!("login cannot enroll a second factor")
    }

    fn verify(&self, secret: &[u8], code: &str, _: u64) -> Result<TotpVerification, DomainError> {
        let mut state = self.0.lock().unwrap();
        state.factor_calls += 1;
        if let Some(change) = state.after_factor.take() {
            state.change(change);
        }
        Ok(if secret == [7; 20] && code == "123456" {
            TotpVerification::Accepted
        } else {
            TotpVerification::Rejected
        })
    }

    fn current_code(&self, _: &[u8], _: u64) -> Result<String, DomainError> {
        Ok("123456".into())
    }
}

impl RecoveryCodeGenerator for Env {
    fn generate(&self, _: usize) -> Result<Vec<Zeroizing<String>>, DomainError> {
        panic!("login cannot generate replacement recovery codes")
    }
}

impl SecretProtector for Env {
    fn protect(&self, _: UserId, _: &[u8]) -> Result<Vec<u8>, ApplicationError> {
        panic!("login cannot replace a protected secret")
    }

    fn expose(&self, _: UserId, value: &[u8]) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
        self.edit(|s| s.secret_exposures += 1);
        Ok(Zeroizing::new(value.to_vec()))
    }
}

impl AuditLog for Env {
    fn append(
        &mut self,
        actor: &str,
        action: &str,
        resource: &str,
        timestamp: OffsetDateTime,
    ) -> Result<ChainedEvent, DomainError> {
        let mut state = self.0.lock().unwrap();
        if state.fail_audit {
            return Err(DomainError::AuditStorageFailure("unavailable".into()));
        }
        let event = ChainedEvent {
            event: AuditEvent::new(
                state.events.len() as u64,
                timestamp,
                actor,
                action,
                resource,
            ),
            chain: Sha256Digest::from_array([0; 32]),
        };
        state.events.push(event.clone());
        Ok(event)
    }

    fn load_all(&self) -> Result<Vec<ChainedEvent>, DomainError> {
        Ok(self.read(|s| s.events.clone()))
    }
}
