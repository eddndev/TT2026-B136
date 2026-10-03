use domain::{crypto::Signature, identity::UserId, owner_certificates::Uuid};

use super::{IdentityPorts, IdentityService, CHALLENGE_TTL_SECONDS};
use crate::{
    identity::{
        certificate_login::{
            validation, CertificateLoginChallenge, CertificateLoginPorts, CertificateMfaChallenge,
            StoredCertificateLogin,
        },
        LoginChallenge, LoginChallengeIdentity, SessionPolicy,
    },
    ApplicationError,
};

impl IdentityService {
    /// Enables certificate first-factor proof without changing password behavior.
    pub fn with_certificate_login(
        ports: IdentityPorts,
        policy: SessionPolicy,
        certificate_login: CertificateLoginPorts,
    ) -> Self {
        let mut service = Self::with_session_policy(ports, policy);
        service.certificate_login = Some(certificate_login);
        service
    }

    /// Captures current public authority and a one-use login-specific statement.
    pub fn start_certificate_login(
        &self,
        owner: UserId,
        binding: Uuid,
    ) -> Result<CertificateLoginChallenge, ApplicationError> {
        let ports = self.certificate_ports()?;
        ports.runtime.admit_start(owner, binding)?;
        let context = self.certificate_context(owner, binding)?;
        let nonce = ports.runtime.nonce()?;
        let now = self.now_seconds();
        let origin = validation::provenance(&context, ports.hasher.as_ref(), now)?;
        let expires = now
            .checked_add(CHALLENGE_TTL_SECONDS as i64)
            .ok_or(ApplicationError::InvalidCredentials)?
            .min(origin.valid_until_unix_seconds);
        let statement = validation::statement(&context, nonce, now, expires)?;
        let capture = StoredCertificateLogin {
            context,
            statement: statement.clone(),
        };
        let token = ports.runtime.create(&capture, (expires - now) as u64)?;
        let admission = (|| {
            self.audit(
                &origin.principal.email,
                "identity.certificate_login_started",
                &owner.to_string(),
            )?;
            self.confirm_certificate_capture(&capture)?;
            let final_now = self.now_seconds();
            if !capture.statement.is_live_at(final_now) {
                return Err(ApplicationError::InvalidCredentials);
            }
            expires
                .checked_sub(final_now)
                .and_then(|value| u64::try_from(value).ok())
                .ok_or(ApplicationError::InvalidCredentials)
        })();
        let expires_in_seconds = match admission {
            Ok(remaining) => remaining,
            Err(error) => {
                ports.runtime.take(&token).map_err(|_| {
                    ApplicationError::Port(
                        "certificate challenge cleanup failed after rejected admission".into(),
                    )
                })?;
                return Err(error);
            }
        };
        Ok(CertificateLoginChallenge {
            challenge_token: token,
            statement,
            expires_in_seconds,
        })
    }

    /// Consumes a proof before RSA and grants only a separate mandatory MFA step.
    pub fn prove_certificate_login(
        &self,
        token: &str,
        signature: &[u8],
    ) -> Result<LoginChallenge, ApplicationError> {
        let ports = self.certificate_ports()?;
        ports.runtime.admit_proof(token)?;
        let capture = ports
            .runtime
            .take(token)?
            .ok_or(ApplicationError::InvalidCredentials)?;
        if signature.len() != 384 {
            return Err(ApplicationError::InvalidCredentials);
        }
        let signature = Signature::from_bytes(signature.to_vec())
            .map_err(|_| ApplicationError::InvalidCredentials)?;
        self.confirm_certificate_capture(&capture)?;
        let at = self.now_seconds();
        validation::capture(&capture, ports.hasher.as_ref(), at)?;
        let check = ports.verifier.verify_login(
            &capture.statement,
            &capture.context.certificate.der,
            &signature,
            &capture.context.trust,
            at,
        )?;
        let origin = validation::evidence(&capture, &signature, &check, ports.hasher.as_ref(), at)?;
        self.confirm_certificate_capture(&capture)?;
        let now = self.now_seconds();
        validation::capture(&capture, ports.hasher.as_ref(), now)?;
        if now < at || !capture.statement.is_live_at(now) {
            return Err(ApplicationError::InvalidCredentials);
        }
        let expires = now
            .checked_add(CHALLENGE_TTL_SECONDS as i64)
            .ok_or(ApplicationError::InvalidCredentials)?
            .min(origin.valid_until_unix_seconds);
        let challenge = CertificateMfaChallenge {
            identity: LoginChallengeIdentity {
                user_id: origin.principal.id,
                auth_generation: origin.auth_generation,
            },
            provenance: origin,
            expires_at_unix_seconds: expires,
        };
        let mfa_token = self
            .ports
            .sessions
            .create_certificate_challenge(&challenge)?;
        let admission = (|| {
            self.audit(
                &challenge.provenance.principal.email,
                "identity.certificate_accepted",
                &challenge.identity.user_id.to_string(),
            )?;
            self.confirm_certificate_capture(&capture)?;
            let final_now = self.now_seconds();
            if final_now < at || final_now >= expires || !capture.statement.is_live_at(final_now) {
                return Err(ApplicationError::InvalidCredentials);
            }
            expires
                .checked_sub(final_now)
                .and_then(|value| u64::try_from(value).ok())
                .ok_or(ApplicationError::InvalidCredentials)
        })();
        let expires_in_seconds = match admission {
            Ok(remaining) => remaining,
            Err(error) => {
                self.ports
                    .sessions
                    .take_mfa_challenge(&mfa_token)
                    .map_err(|_| {
                        ApplicationError::Port(
                            "certificate MFA cleanup failed after rejected admission".into(),
                        )
                    })?;
                return Err(error);
            }
        };
        Ok(LoginChallenge {
            challenge_token: mfa_token,
            expires_in_seconds,
        })
    }
}
