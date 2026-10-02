use application::identity::{
    EnrollmentResult, IdentityWorkflow, LoginChallenge, Principal, SessionResult, SessionStatus,
};
use application::ApplicationError;
use domain::identity::{Permission, Role};

pub struct StubIdentity;

impl IdentityWorkflow for StubIdentity {
    fn bootstrap_owner(
        &self,
        _email: &str,
        _password: &str,
    ) -> Result<EnrollmentResult, ApplicationError> {
        Err(ApplicationError::BootstrapClosed)
    }

    fn create_user(
        &self,
        _token: &str,
        _email: &str,
        _password: &str,
        _role: Role,
    ) -> Result<EnrollmentResult, ApplicationError> {
        Err(ApplicationError::InvalidInput("unused".to_string()))
    }

    fn start_login(
        &self,
        _email: &str,
        _password: &str,
    ) -> Result<LoginChallenge, ApplicationError> {
        Err(ApplicationError::InvalidCredentials)
    }

    fn complete_totp(
        &self,
        _challenge_token: &str,
        _code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        Err(ApplicationError::MfaRejected)
    }

    fn complete_recovery(
        &self,
        _challenge_token: &str,
        _code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        Err(ApplicationError::MfaRejected)
    }

    fn authenticate(&self, token: &str) -> Result<Principal, ApplicationError> {
        panic!("document authentication must run in the workflow: {token}")
    }

    fn session_status(&self, _token: &str) -> Result<SessionStatus, ApplicationError> {
        unreachable!("document routes do not query session status")
    }

    fn record_activity(&self, _token: &str) -> Result<SessionStatus, ApplicationError> {
        unreachable!("document routes do not record session activity")
    }

    fn authorize(
        &self,
        token: &str,
        permission: Permission,
    ) -> Result<Principal, ApplicationError> {
        let principal = self.authenticate(token)?;
        if principal.role.allows(permission) {
            Ok(principal)
        } else {
            Err(ApplicationError::PermissionDenied)
        }
    }

    fn logout(&self, _access_token: &str) -> Result<(), ApplicationError> {
        Ok(())
    }
}
