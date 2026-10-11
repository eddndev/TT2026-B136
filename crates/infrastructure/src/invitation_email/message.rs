use super::InvitationEmailConfiguration;
use application::identity::invitations::InvitationDeliveryLease;
use application::ApplicationError;
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
    configuration: &InvitationEmailConfiguration,
    lease: &InvitationDeliveryLease,
    token: &[u8],
) -> Result<Body, ApplicationError> {
    let token = Zeroizing::new(URL_SAFE_NO_PAD.encode(token));
    let expiry = lease
        .expires_at
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| unavailable())?;
    let text = Zeroizing::new(format!(
        "Recibiste una invitacion para crear tu cuenta en Qadra.\n\n{}#invitation={}\n\nEl enlace vence {expiry}.\n\nDefine tu contrasena y confirma tu segundo factor para completar el registro.\n\nSi no esperabas esta invitacion, ignora este mensaje.",
        configuration.public_url.as_str(), token.as_str(),
    ));
    let message = Message {
        from: &configuration.from_email,
        to: [&lease.email],
        subject: "Qadra: invitacion para crear tu cuenta",
        text: text.as_str(),
    };
    let mut bytes = Zeroizing::new(Vec::new());
    serde_json::to_writer(&mut *bytes, &message).map_err(|_| unavailable())?;
    let length = u64::try_from(bytes.len()).map_err(|_| unavailable())?;
    // The reader clears our serialized copy even after transport failure.
    // reqwest and TLS can retain their own internal copies beyond this reader.
    Ok(Body::sized(Cursor::new(bytes), length))
}

fn unavailable() -> ApplicationError {
    ApplicationError::Port("invitation email preparation failed".into())
}
