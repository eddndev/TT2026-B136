use application::ApplicationError;
use reqwest::Url;

/// Explicit sender and fixed public destination for password recovery messages.
pub struct PasswordResetEmailConfiguration {
    pub(super) from_email: String,
    pub(super) public_url: Url,
}

impl PasswordResetEmailConfiguration {
    /// Accepts one bare sender address and an HTTPS origin's root page.
    pub fn new(from_email: String, public_url: String) -> Result<Self, ApplicationError> {
        if !address(&from_email)
            || public_url.len() > 2048
            || !public_url.starts_with("https://")
            || !public_url.bytes().all(|byte| byte.is_ascii_graphic())
            || public_url.contains('\\')
        {
            return Err(invalid());
        }
        let public_url = Url::parse(&public_url).map_err(|_| invalid())?;
        if public_url.scheme() != "https"
            || public_url.host_str().is_none()
            || !public_url.username().is_empty()
            || public_url.password().is_some()
            || public_url.query().is_some()
            || public_url.fragment().is_some()
            || public_url.path() != "/"
        {
            return Err(invalid());
        }
        Ok(Self {
            from_email,
            public_url,
        })
    }
}

pub(super) fn address(value: &str) -> bool {
    if value.len() > 254
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
        || value.bytes().any(|byte| b"<>,;:\"\\[]".contains(&byte))
    {
        return false;
    }
    value.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty() && !domain.is_empty() && !domain.contains('@')
    })
}

pub(super) fn invalid() -> ApplicationError {
    ApplicationError::InvalidConfiguration("invalid password recovery email configuration".into())
}
