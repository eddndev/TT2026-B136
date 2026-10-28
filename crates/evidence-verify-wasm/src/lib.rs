//! Browser entry point of the evidence validator.
//!
//! The Web Worker passes the package bytes and the policy as JSON; this crate
//! parses the policy, runs [`evidence_verify::validate`] and returns the
//! report as a JSON string. Only the policy can be rejected: once it parses,
//! every package, however malformed, yields a report. The JSON contract is
//! `docs/adr/0085-in-browser-evidence-validation.md`.

pub mod policy_json;
pub mod report_json;

#[cfg(target_arch = "wasm32")]
mod wasm;

use policy_json::{parse_policy, PolicyError};

/// The version reported as the engine version; the engine crate has none of
/// its own, so the wrapper version identifies the build.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The engine name reported with [`ENGINE_VERSION`].
pub const ENGINE_NAME: &str = "evidence-verify";

/// Validate `zip` under the policy encoded in `policy_json` and return the
/// report as JSON, or the reason the policy was rejected.
pub fn validate_json(zip: &[u8], policy_json: &str) -> Result<String, PolicyError> {
    let policy = parse_policy(policy_json)?;
    let report = evidence_verify::validate(zip, &policy);
    let mut value = report_json::report_value(&report);
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "engine".to_owned(),
            serde_json::json!({"name": ENGINE_NAME, "version": ENGINE_VERSION}),
        );
    }
    Ok(value.to_string())
}
