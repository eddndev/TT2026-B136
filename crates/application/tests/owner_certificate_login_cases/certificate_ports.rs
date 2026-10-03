use application::credential_trust::CredentialTrustSnapshot;
use domain::{
    crypto::{CredentialCheck, DocumentHasher, Sha256Digest, Signature},
    identity::UserId,
    owner_certificate_login::{LoginNonce, LoginStatement},
};
use uuid::Uuid;

use super::{
    ApplicationError, CertificateLoginContext, CheckFault, Env, Hasher, OwnerLoginAuthority,
    OwnerLoginRuntime, OwnerLoginVerifier, StoredCertificateLogin,
};

impl OwnerLoginAuthority for Env {
    fn load(
        &self,
        owner: UserId,
        binding: Uuid,
    ) -> Result<Option<CertificateLoginContext>, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.authority_reads += 1;
        if state.fail_authority {
            return Err(ApplicationError::Port("authority unavailable".into()));
        }
        Ok(state
            .context
            .clone()
            .filter(|value| value.account.principal.id == owner && value.binding_id == binding))
    }
}

impl OwnerLoginRuntime for Env {
    fn admit_start(&self, _: UserId, _: Uuid) -> Result<(), ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.start_admissions += 1;
        if state.fail_start {
            Err(ApplicationError::AccountLocked)
        } else {
            Ok(())
        }
    }

    fn admit_proof(&self, _: &str) -> Result<(), ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.proof_admissions += 1;
        if state.fail_proof {
            Err(ApplicationError::AccountLocked)
        } else {
            Ok(())
        }
    }

    fn nonce(&self) -> Result<LoginNonce, ApplicationError> {
        Ok(LoginNonce::from_bytes(&[0x66; 32]).unwrap())
    }

    fn create(&self, value: &StoredCertificateLogin, ttl: u64) -> Result<String, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        assert_eq!(value.statement.issued_at_unix_seconds(), state.now);
        assert_eq!(
            ttl as i64,
            value.statement.expires_at_unix_seconds() - state.now
        );
        assert!((1..=300).contains(&ttl));
        let token = state.token("proof");
        state.proofs.insert(token.clone(), value.clone());
        Ok(token)
    }

    fn take(&self, token: &str) -> Result<Option<StoredCertificateLogin>, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.proof_takes += 1;
        let value = state.proofs.remove(token);
        Ok(value.filter(|value| value.statement.is_live_at(state.now)))
    }
}

impl OwnerLoginVerifier for Env {
    fn verify_login(
        &self,
        statement: &LoginStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.verifier_calls += 1;
        assert!(
            state
                .proofs
                .values()
                .all(|value| value.statement != *statement),
            "a proof must be consumed before cryptographic verification"
        );
        let captured = state.context.as_ref().unwrap();
        assert_eq!(certificate, captured.certificate.der);
        assert_eq!(trust, &captured.trust);
        assert_eq!(at, state.now);
        let mut check = CredentialCheck {
            certificate: captured.certificate.clone(),
            trust: trust.inspection.clone(),
            statement_digest: Hasher.hash_bytes(&statement.canonical_bytes()),
            signature: signature.clone(),
            checked_at: at,
            valid_from: 100,
            valid_until: 2000,
        };
        if state.fail_verify {
            return Err(ApplicationError::InvalidCredentials);
        }
        match state.check_fault {
            Some(CheckFault::Digest) => check.statement_digest = Sha256Digest::from_array([9; 32]),
            Some(CheckFault::Signature) => {
                check.signature = Signature::from_bytes(vec![9; 384]).unwrap()
            }
            Some(CheckFault::Certificate) => check.certificate.der.push(0),
            Some(CheckFault::Trust) => check.trust.crl_number += 1,
            Some(CheckFault::Time) => check.checked_at -= 1,
            Some(CheckFault::Window) => check.valid_until = 2001,
            None => {}
        }
        if let Some(change) = state.after_verify.take() {
            state.change(change);
        }
        Ok(check)
    }
}
