//! Invitations have an explicit independent email activation switch.
use application::ApplicationError;
use infrastructure::invitation_email::InvitationEmailConfiguration;
use zeroize::Zeroizing;

#[derive(Debug, Default, clap::Args)]
pub struct InvitationOptions {
    /// Enable invitations only after configuring their public email channel.
    #[arg(
        id = "invitations_enabled",
        long = "invitations-enabled",
        env = "TT_INVITATIONS_ENABLED"
    )]
    pub enabled: bool,
    #[arg(
        id = "invitation_email_from",
        long = "invitation-email-from",
        env = "INVITATION_EMAIL_FROM"
    )]
    pub from_email: Option<String>,
    #[arg(
        id = "invitation_public_url",
        long = "invitation-public-url",
        env = "INVITATION_PUBLIC_URL"
    )]
    pub public_url: Option<String>,
}
pub(crate) struct InvitationSettings {
    pub configuration: InvitationEmailConfiguration,
    pub api_key: Zeroizing<String>,
}
impl InvitationSettings {
    pub(crate) fn load(options: &InvitationOptions) -> anyhow::Result<Option<Self>> {
        let key = if options.enabled {
            match std::env::var("RESEND_API_KEY") {
                Ok(value) => Some(Zeroizing::new(value)),
                Err(std::env::VarError::NotPresent) => None,
                Err(_) => anyhow::bail!("invitation email secret must be valid text"),
            }
        } else {
            None
        };
        Ok(Self::resolve(
            InvitationOptions {
                enabled: options.enabled,
                from_email: options.from_email.clone(),
                public_url: options.public_url.clone(),
            },
            key,
        )?)
    }
    pub(crate) fn resolve(
        options: InvitationOptions,
        api_key: Option<Zeroizing<String>>,
    ) -> Result<Option<Self>, ApplicationError> {
        if !options.enabled {
            return Ok(None);
        }
        let invalid = || {
            ApplicationError::InvalidConfiguration("incomplete invitation email settings".into())
        };
        let api_key = api_key.ok_or_else(invalid)?;
        if api_key.is_empty()
            || api_key.len() > 1024
            || !api_key.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err(invalid());
        }
        let configuration = InvitationEmailConfiguration::new(
            options.from_email.ok_or_else(invalid)?,
            options.public_url.ok_or_else(invalid)?,
        )?;
        Ok(Some(Self {
            configuration,
            api_key,
        }))
    }
}
