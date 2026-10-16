//! Narrow browser bindings. Only public results and encrypted backups cross this boundary.
use wasm_bindgen::prelude::*;
use zeroize::Zeroizing;

fn error(value: crate::CryptoError) -> JsValue {
    JsValue::from_str(value.0)
}

#[wasm_bindgen]
pub fn generate_key(
    password: String,
    common_name: &str,
    organization: &str,
    country: &str,
) -> Result<String, JsValue> {
    let password = Zeroizing::new(password);
    let value = crate::generate(&password, common_name, organization, country).map_err(error)?;
    serde_json::to_string(&value).map_err(|_| JsValue::from_str("input"))
}

#[wasm_bindgen]
pub fn sign_bytes(
    encrypted_pem: &str,
    password: String,
    certificate: &[u8],
    bytes: &[u8],
) -> Result<Vec<u8>, JsValue> {
    let password = Zeroizing::new(password);
    crate::sign(encrypted_pem, &password, certificate, bytes).map_err(error)
}

#[wasm_bindgen]
pub fn inspect_certificate(certificate: &[u8]) -> Result<String, JsValue> {
    let value = crate::inspect(certificate).map_err(error)?;
    serde_json::to_string(&value).map_err(|_| JsValue::from_str("certificate"))
}

#[wasm_bindgen]
pub fn check_key_backup(
    encrypted_pem: &str,
    password: String,
    request_pem: &str,
) -> Result<bool, JsValue> {
    let password = Zeroizing::new(password);
    crate::check_key(encrypted_pem, &password, request_pem).map_err(error)
}
