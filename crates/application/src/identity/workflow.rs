//! Dynamic inbound identity boundary implementation.

use domain::identity::{Permission, Role};

use super::{
    EnrollmentResult, IdentityService, IdentityWorkflow, LoginChallenge, Principal, SessionResult,
};
use crate::ApplicationError;

impl IdentityWorkflow for IdentityService {
    fn bootstrap_owner(
        &self,
        email: &str,
        password: &str,
    ) -> Result<EnrollmentResult, ApplicationError> {
        Self::bootstrap_owner(self, email, password)
    }

    fn create_user(
        &self,
        access_token: &str,
        email: &str,
        password: &str,
        role: Role,
    ) -> Result<EnrollmentResult, ApplicationError> {
        Self::create_user(self, access_token, email, password, role)
    }

    fn start_login(&self, email: &str, password: &str) -> Result<LoginChallenge, ApplicationError> {
        Self::start_login(self, email, password)
    }

    fn complete_totp(
        &self,
        challenge_token: &str,
        code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        Self::complete_totp(self, challenge_token, code)
    }

    fn complete_recovery(
        &self,
        challenge_token: &str,
        code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        Self::complete_recovery(self, challenge_token, code)
    }

    fn authenticate(&self, access_token: &str) -> Result<Principal, ApplicationError> {
        Self::authenticate(self, access_token)
    }

    fn authorize(
        &self,
        access_token: &str,
        permission: Permission,
    ) -> Result<Principal, ApplicationError> {
        Self::authorize(self, access_token, permission)
    }

    fn logout(&self, access_token: &str) -> Result<(), ApplicationError> {
        Self::logout(self, access_token)
    }
}
