//! Local administrative enrollment with private delivery and confirmed TOTP.
use std::{io::Write, path::Path};

use anyhow::bail;
use application::identity::{EnrollmentResult, SecretProtector, SessionPolicy, UserRepository};
use domain::{
    clock::OffsetDateTime,
    crypto::{TotpProvider, TotpVerification},
    identity::UserId,
};
use infrastructure::{provision_initial_ca_operator, AesGcmSecretProtector, TotpRsProvider};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use super::{composition, config};

#[path = "enrollment_io.rs"]
mod private_io;
use private_io::{PrivateDelivery, PrivateInput};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Setup {
    database_name: String,
    operator_email: String,
    kek_base64: String,
    admin: Admin,
    runtime: Runtime,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Admin {
    database_url: String,
    database_role: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Runtime {
    database_url: String,
    database_role: String,
    redis_url: String,
}

impl Drop for Setup {
    fn drop(&mut self) {
        self.kek_base64.zeroize();
    }
}

impl Drop for Admin {
    fn drop(&mut self) {
        self.database_url.zeroize();
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.database_url.zeroize();
        self.redis_url.zeroize();
    }
}

#[derive(Serialize)]
struct Delivery<'a> {
    operator_id: UserId,
    operator_email: &'a str,
    totp_secret_base32: &'a str,
    otpauth_uri: &'a str,
    recovery_codes: Vec<&'a str>,
}

#[derive(Serialize)]
struct Receipt<'a> {
    operator_id: UserId,
    operator_email: &'a str,
}

/// Prepares locally, verifies private delivery and MFA, then commits one account.
pub fn run(setup_path: &Path, enrollment_out: &Path) -> anyhow::Result<()> {
    let bytes = config::read_private(setup_path)?;
    let setup: Setup = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("CA enrollment setup format is invalid"))?;
    if setup.database_name.is_empty()
        || setup.admin.database_role.is_empty()
        || setup.runtime.database_role.is_empty()
        || setup.admin.database_role == setup.runtime.database_role
    {
        bail!("CA enrollment requires separate explicit administrative and runtime authority");
    }
    let kek = config::decode_kek(&setup.kek_base64)?;
    let (users, identity) = composition::open_identity(
        &setup.runtime.database_url,
        &setup.database_name,
        &setup.runtime.database_role,
        &setup.runtime.redis_url,
        kek.clone(),
        SessionPolicy::default(),
    )?;
    if users.has_users()? {
        bail!("initial CA enrollment is already closed");
    }
    let mut delivery = PrivateDelivery::create(enrollment_out)?;
    let input = PrivateInput::open()?;
    let password = input.line("Password", 1024)?;
    let repeated = input.line("Repeat password", 1024)?;
    if *password != *repeated {
        bail!("private enrollment passwords do not match");
    }
    let (record, enrollment) = identity.prepare_ca_operator(&setup.operator_email, &password)?;
    drop(password);
    drop(repeated);
    delivery.persist(encode_delivery(&enrollment)?)?;
    input
        .notice("Private enrollment file saved. Read it locally and enroll your authenticator.")?;
    let code = input.line("Authenticator code", 6)?;
    if code.len() != 6 || !code.bytes().all(|value| value.is_ascii_digit()) {
        bail!("CA enrollment second factor was not confirmed");
    }
    let secret =
        AesGcmSecretProtector::new(kek)?.expose(record.id, &record.protected_totp_secret)?;
    let now = OffsetDateTime::now_utc();
    if TotpRsProvider::new().verify(&secret, &code, now.unix_timestamp().try_into()?)?
        != TotpVerification::Accepted
    {
        bail!("CA enrollment second factor was not confirmed");
    }
    delivery.verify()?;
    provision_initial_ca_operator(
        &setup.admin.database_url,
        &setup.database_name,
        &setup.admin.database_role,
        record,
        now,
    )?;
    let receipt = Receipt {
        operator_id: enrollment.principal.id,
        operator_email: &enrollment.principal.email,
    };
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, &receipt)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}

fn encode_delivery(enrollment: &EnrollmentResult) -> anyhow::Result<Zeroizing<Vec<u8>>> {
    let delivery = Delivery {
        operator_id: enrollment.principal.id,
        operator_email: &enrollment.principal.email,
        totp_secret_base32: &enrollment.totp_secret_base32,
        otpauth_uri: &enrollment.otpauth_uri,
        recovery_codes: enrollment
            .recovery_codes
            .iter()
            .map(|code| code.as_str())
            .collect(),
    };
    let mut bytes = Zeroizing::new(Vec::new());
    serde_json::to_writer_pretty(&mut *bytes, &delivery)?;
    bytes.push(b'\n');
    Ok(bytes)
}
