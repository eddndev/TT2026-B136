use super::PasswordResetEmailConfiguration;
use application::{identity::password_reset::ResetEnvelope, ApplicationError};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use reqwest::blocking::Body;
use serde::Serialize;
use std::io::Cursor;
use time::{format_description::well_known::Rfc3339, UtcOffset};
use zeroize::Zeroizing;

#[derive(Serialize)]
struct Message<'a> {
    from: &'a str,
    to: [&'a str; 1],
    subject: &'static str,
    text: &'a str,
}

pub(super) fn body(
    configuration: &PasswordResetEmailConfiguration,
    envelope: &ResetEnvelope,
) -> Result<Body, ApplicationError> {
    let token = Zeroizing::new(URL_SAFE_NO_PAD.encode(envelope.token.as_ref()));
    let expiry = envelope
        .expires_at
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| unavailable())?;
    let text = Zeroizing::new(format!(
        "Solicitaste restablecer tu contrasena en Qadra.\n\n{}#password-reset={}\n\nEl enlace vence {expiry}.\n\nSi no lo solicitaste, ignora este mensaje.",
        configuration.public_url.as_str(), token.as_str(),
    ));
    let message = Message {
        from: &configuration.from_email,
        to: [&envelope.email],
        subject: "Qadra: restablece tu contrasena",
        text: text.as_str(),
    };
    let mut bytes = Zeroizing::new(Vec::new());
    serde_json::to_writer(&mut *bytes, &message).map_err(|_| unavailable())?;
    let length = u64::try_from(bytes.len()).map_err(|_| unavailable())?;
    // The reader owns and clears our serialized copy, including on transport failure.
    // reqwest and TLS may keep their own internal copies beyond this reader.
    Ok(Body::sized(Cursor::new(bytes), length))
}

pub(super) fn submission_id() -> Result<String, ApplicationError> {
    let mut random = [0u8; 16];
    getrandom::getrandom(&mut random).map_err(|_| unavailable())?;
    Ok(uuid::Builder::from_random_bytes(random)
        .into_uuid()
        .to_string())
}

fn unavailable() -> ApplicationError {
    ApplicationError::Port("password recovery email preparation failed".into())
}
