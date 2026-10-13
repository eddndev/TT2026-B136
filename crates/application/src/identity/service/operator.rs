//! Local administrative credential preparation; this never persists an account.
use super::{normalize_email, validate_password, IdentityService};
use crate::identity::{EnrollmentResult, UserRecord};
use crate::ApplicationError;
use domain::identity::Role;

impl IdentityService {
    /// Prepares credentials for the separately provisioned CA identity store.
    ///
    /// The administrative composition persists the result only in qadra_ca.
    /// No office user, session, privilege or public bootstrap is created here.
    pub fn prepare_ca_operator(
        &self,
        email: &str,
        password: &str,
    ) -> Result<(UserRecord, EnrollmentResult), ApplicationError> {
        let email = normalize_email(email)?;
        validate_password(password)?;
        self.enroll(&email, password, Role::Client)
    }
}
