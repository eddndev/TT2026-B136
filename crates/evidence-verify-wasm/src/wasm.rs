//! The WebAssembly export used by the evidence validation Web Worker.

use wasm_bindgen::prelude::*;

/// Validate `zip` under `policy_json` and return the report as JSON. The
/// only error is a rejected policy, returned as its stable code.
#[wasm_bindgen]
pub fn validate(zip: &[u8], policy_json: &str) -> Result<String, JsValue> {
    crate::validate_json(zip, policy_json).map_err(|error| JsValue::from_str(error.code()))
}
