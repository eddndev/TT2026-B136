use anyhow::Context;
use std::fs;

pub(super) fn read(path: &std::path::Path, label: &str) -> anyhow::Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("cannot read {label} at {}", path.display()))
}

pub(super) fn required_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("{name} must be set for the HTTP application"))
}
