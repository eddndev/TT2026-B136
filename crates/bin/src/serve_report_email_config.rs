//! Explicit, independent activation of ready-report email delivery.
use application::ApplicationError;
use infrastructure::case_report_email::CaseReportEmailConfiguration;
use zeroize::Zeroizing;

pub(crate) struct ReportEmailSettings {
    pub configuration: CaseReportEmailConfiguration,
    pub api_key: Zeroizing<String>,
}

impl ReportEmailSettings {
    pub(crate) fn load() -> Result<Option<Self>, ApplicationError> {
        Self::load_with(|name| match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(invalid()),
        })
    }

    pub(crate) fn load_with(
        mut read: impl FnMut(&str) -> Result<Option<String>, ApplicationError>,
    ) -> Result<Option<Self>, ApplicationError> {
        match read("TT_REPORT_EMAIL_ENABLED")?.as_deref() {
            None | Some("false") => return Ok(None),
            Some("true") => (),
            Some(_) => return Err(invalid()),
        }
        let from = read("REPORT_EMAIL_FROM")?.ok_or_else(invalid)?;
        let public_url = read("REPORT_PUBLIC_URL")?.ok_or_else(invalid)?;
        let api_key = Zeroizing::new(read("RESEND_API_KEY")?.ok_or_else(invalid)?);
        if api_key.is_empty()
            || api_key.len() > 1024
            || !api_key.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(invalid());
        }
        let configuration = CaseReportEmailConfiguration::new(from, public_url)?;
        Ok(Some(Self {
            configuration,
            api_key,
        }))
    }
}

fn invalid() -> ApplicationError {
    ApplicationError::InvalidConfiguration("invalid report email settings".into())
}
