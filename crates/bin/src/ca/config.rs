//! Independently supplied CA service settings; never expose credentials in errors.
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::{fs::OpenOptions, io::Read, net::SocketAddr, path::Path};

use anyhow::{bail, Context};
use application::{ca_access::OperatorIdentity, identity::SessionPolicy};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::identity::UserId;
use serde::Deserialize;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationSettings {
    pub python: std::path::PathBuf,
    pub worker: std::path::PathBuf,
    pub root: std::path::PathBuf,
    pub inspector: std::path::PathBuf,
    pub cli: std::path::PathBuf,
    pub scripts: std::path::PathBuf,
    pub expected_root: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub bind: SocketAddr,
    pub database_url: String,
    pub database_name: String,
    pub database_role: String,
    pub redis_url: String,
    kek_base64: String,
    operator_id: Uuid,
    operator_email: String,
    session_ttl_seconds: u64,
    idle_ttl_seconds: Option<u64>,
    pub operations: Option<OperationSettings>,
}

impl Drop for Settings {
    fn drop(&mut self) {
        self.database_url.zeroize();
        self.redis_url.zeroize();
        self.kek_base64.zeroize();
    }
}

pub fn load(path: &Path) -> anyhow::Result<Settings> {
    let bytes = read_private(path)?;
    let settings: Settings = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("CA settings format is invalid"))?;
    if !settings.bind.ip().is_loopback()
        || settings.bind.port() == 0
        || settings.database_name.is_empty()
        || settings.database_role.is_empty()
    {
        bail!("CA requires a loopback listener and explicit database authority");
    }
    settings.operator()?;
    settings.policy()?;
    settings.kek()?;
    Ok(settings)
}

pub(super) fn read_private(path: &Path) -> anyhow::Result<Zeroizing<Vec<u8>>> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(path)
        .context("CA settings are unavailable")?;
    let info = file
        .metadata()
        .context("CA settings metadata is unavailable")?;
    if !info.is_file()
        || info.mode() & 0o077 != 0
        || info.uid() != rustix::process::geteuid().as_raw()
    {
        bail!("CA settings must be an owner-only regular file");
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(65537)
        .read_to_end(&mut bytes)
        .context("CA settings cannot be read")?;
    if bytes.is_empty() || bytes.len() > 65536 {
        bail!("CA settings exceed the configuration bound");
    }
    Ok(bytes)
}

impl Settings {
    pub fn operator(&self) -> anyhow::Result<OperatorIdentity> {
        OperatorIdentity::new(UserId::from_uuid(self.operator_id), &self.operator_email)
            .map_err(|_| anyhow::anyhow!("CA operator configuration is invalid"))
    }

    pub fn policy(&self) -> anyhow::Result<SessionPolicy> {
        SessionPolicy::new(self.session_ttl_seconds, self.idle_ttl_seconds)
            .map_err(|_| anyhow::anyhow!("CA session policy is invalid"))
    }

    pub fn kek(&self) -> anyhow::Result<Zeroizing<Vec<u8>>> {
        decode_kek(&self.kek_base64)
    }
}

pub(super) fn decode_kek(encoded: &str) -> anyhow::Result<Zeroizing<Vec<u8>>> {
    let bytes = Zeroizing::new(
        STANDARD
            .decode(encoded)
            .map_err(|_| anyhow::anyhow!("CA credential protection key is invalid"))?,
    );
    if bytes.len() != 32 {
        bail!("CA credential protection key must have 32 bytes");
    }
    Ok(bytes)
}
