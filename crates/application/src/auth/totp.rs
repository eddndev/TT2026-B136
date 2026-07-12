//! Use cases around the time-based second factor.

use domain::crypto::password::PasswordHasher;
use domain::crypto::recovery::{RecoveryCodeGenerator, RecoveryCodeSet, RECOVERY_CODE_COUNT};
use domain::crypto::totp::{TotpEnrollment, TotpProvider, TotpVerification};
use zeroize::Zeroizing;

use crate::error::ApplicationError;

/// Everything produced by a second-factor enrollment.
pub struct TotpEnrollmentBundle {
    /// Shared secret, its base32 form, and the provisioning URI.
    pub enrollment: TotpEnrollment,
    /// The recovery codes in plain form. This is the only moment they
    /// exist in clear; they are shown to the user and then dropped.
    pub plain_recovery_codes: Vec<Zeroizing<String>>,
    /// The storable form of the recovery codes: hashes only.
    pub recovery_codes: RecoveryCodeSet,
}

/// Enrolls a user: draws the shared secret and issues the recovery codes.
pub struct EnrollTotp<T, G, H>
where
    T: TotpProvider,
    G: RecoveryCodeGenerator,
    H: PasswordHasher,
{
    totp: T,
    generator: G,
    hasher: H,
}

impl<T, G, H> EnrollTotp<T, G, H>
where
    T: TotpProvider,
    G: RecoveryCodeGenerator,
    H: PasswordHasher,
{
    pub fn new(totp: T, generator: G, hasher: H) -> Self {
        Self {
            totp,
            generator,
            hasher,
        }
    }

    pub fn execute(&self, account_email: &str) -> Result<TotpEnrollmentBundle, ApplicationError> {
        let enrollment = self.totp.enroll(account_email)?;
        let plain_recovery_codes = self.generator.generate(RECOVERY_CODE_COUNT)?;
        let mut hashes = Vec::with_capacity(plain_recovery_codes.len());
        for code in &plain_recovery_codes {
            hashes.push(self.hasher.hash(code)?);
        }
        let recovery_codes = RecoveryCodeSet::from_hashes(hashes)?;
        Ok(TotpEnrollmentBundle {
            enrollment,
            plain_recovery_codes,
            recovery_codes,
        })
    }
}

/// Checks a submitted one-time code at a given unix timestamp.
pub struct VerifyTotp<T: TotpProvider> {
    totp: T,
}

impl<T: TotpProvider> VerifyTotp<T> {
    pub fn new(totp: T) -> Self {
        Self { totp }
    }

    /// A rejected code is a normal outcome, distinct from backend errors.
    pub fn execute(
        &self,
        secret: &[u8],
        code: &str,
        unix_seconds: u64,
    ) -> Result<TotpVerification, ApplicationError> {
        Ok(self.totp.verify(secret, code, unix_seconds)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::crypto::password::PasswordVerification;
    use domain::DomainError;
    use mockall::mock;

    mock! {
        Totp {}
        impl TotpProvider for Totp {
            fn enroll(&self, account_email: &str) -> Result<TotpEnrollment, DomainError>;
            fn verify(
                &self,
                secret: &[u8],
                code: &str,
                unix_seconds: u64,
            ) -> Result<TotpVerification, DomainError>;
            fn current_code(
                &self,
                secret: &[u8],
                unix_seconds: u64,
            ) -> Result<String, DomainError>;
        }
    }

    mock! {
        Generator {}
        impl RecoveryCodeGenerator for Generator {
            fn generate(&self, count: usize) -> Result<Vec<Zeroizing<String>>, DomainError>;
        }
    }

    mock! {
        Hasher {}
        impl PasswordHasher for Hasher {
            fn hash(&self, password: &str) -> Result<String, DomainError>;
            fn verify(
                &self,
                password: &str,
                stored_phc: &str,
            ) -> Result<PasswordVerification, DomainError>;
        }
    }

    fn stub_enrollment() -> TotpEnrollment {
        TotpEnrollment {
            secret: Zeroizing::new(vec![7u8; 20]),
            secret_base32: Zeroizing::new("A7A7A7A7".to_string()),
            otpauth_uri: Zeroizing::new(
                "otpauth://totp/despacho:user%40example.com?secret=A7A7A7A7".to_string(),
            ),
        }
    }

    fn plain_codes(count: usize) -> Vec<Zeroizing<String>> {
        (0..count)
            .map(|i| Zeroizing::new(format!("CODE{i}-CODE{i}")))
            .collect()
    }

    #[test]
    fn enrollment_bundles_provisioning_material_and_eight_recovery_codes() {
        let mut totp = MockTotp::new();
        totp.expect_enroll()
            .withf(|email| email == "user@example.com")
            .times(1)
            .returning(|_| Ok(stub_enrollment()));
        let mut generator = MockGenerator::new();
        generator
            .expect_generate()
            .withf(|count| *count == RECOVERY_CODE_COUNT)
            .times(1)
            .returning(|count| Ok(plain_codes(count)));
        let mut hasher = MockHasher::new();
        hasher
            .expect_hash()
            .times(RECOVERY_CODE_COUNT)
            .returning(|code| Ok(format!("fake:{code}")));

        let bundle = EnrollTotp::new(totp, generator, hasher)
            .execute("user@example.com")
            .unwrap();

        assert_eq!(bundle.enrollment.secret.as_slice(), &[7u8; 20]);
        assert_eq!(bundle.plain_recovery_codes.len(), RECOVERY_CODE_COUNT);
        assert_eq!(bundle.recovery_codes.remaining(), RECOVERY_CODE_COUNT);
        assert_eq!(bundle.plain_recovery_codes[0].as_str(), "CODE0-CODE0");
    }

    #[test]
    fn enrollment_fails_when_the_generator_returns_the_wrong_count() {
        let mut totp = MockTotp::new();
        totp.expect_enroll().returning(|_| Ok(stub_enrollment()));
        let mut generator = MockGenerator::new();
        generator
            .expect_generate()
            .returning(|_| Ok(plain_codes(RECOVERY_CODE_COUNT - 1)));
        let mut hasher = MockHasher::new();
        hasher
            .expect_hash()
            .returning(|code| Ok(format!("fake:{code}")));

        // The bundle carries secrets and deliberately has no Debug impl,
        // so the error is extracted without unwrap_err.
        let err = EnrollTotp::new(totp, generator, hasher)
            .execute("user@example.com")
            .err()
            .unwrap();
        assert!(matches!(
            err,
            ApplicationError::Domain(DomainError::InvalidRecoveryCodeCount {
                expected: RECOVERY_CODE_COUNT,
                actual: 7,
            })
        ));
    }

    #[test]
    fn verify_totp_passes_both_outcomes_through() {
        let mut totp = MockTotp::new();
        totp.expect_verify()
            .withf(|secret, code, at| {
                secret == [7u8; 20] && code == "123456" && *at == 1_111_111_109
            })
            .returning(|_, _, _| Ok(TotpVerification::Accepted));
        totp.expect_verify()
            .withf(|_, code, _| code == "654321")
            .returning(|_, _, _| Ok(TotpVerification::Rejected));

        let use_case = VerifyTotp::new(totp);
        assert_eq!(
            use_case
                .execute(&[7u8; 20], "123456", 1_111_111_109)
                .unwrap(),
            TotpVerification::Accepted
        );
        assert_eq!(
            use_case
                .execute(&[7u8; 20], "654321", 1_111_111_109)
                .unwrap(),
            TotpVerification::Rejected
        );
    }

    #[test]
    fn verify_totp_surfaces_backend_errors() {
        let mut totp = MockTotp::new();
        totp.expect_verify().returning(|_, _, _| {
            Err(DomainError::TotpSecretTooShort {
                minimum: 16,
                actual: 4,
            })
        });
        let err = VerifyTotp::new(totp)
            .execute(&[1u8; 4], "123456", 1_111_111_109)
            .unwrap_err();
        assert!(matches!(
            err,
            ApplicationError::Domain(DomainError::TotpSecretTooShort { .. })
        ));
    }
}
