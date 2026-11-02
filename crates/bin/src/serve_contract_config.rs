//! Contract notice settings. E-mail delivery of notices has its own
//! explicit activation and reuses the ready-report sender, public origin and
//! provider secret; without it, notices stay in storage and the parties see
//! their requests in Qadra. See docs/adr/0086-bilateral-contract-signing.md.
use application::{contracts::NoticeSettings, ApplicationError};
use infrastructure::case_report_email::CaseReportEmailConfiguration;
use zeroize::Zeroizing;

/// The sender stored with notices written while contract e-mail is off.
pub(crate) const DISABLED_SENDER: &str = "notificaciones@qadra.invalid";

pub(crate) struct ContractSettings {
    pub notices: NoticeSettings,
    /// The provider secret, present only when contract e-mail is enabled.
    pub email: Option<Zeroizing<String>>,
}

impl ContractSettings {
    pub(crate) fn load() -> Result<Self, ApplicationError> {
        Self::load_with(|name| match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(invalid()),
        })
    }

    pub(crate) fn load_with(
        mut read: impl FnMut(&str) -> Result<Option<String>, ApplicationError>,
    ) -> Result<Self, ApplicationError> {
        match read("TT_CONTRACT_EMAIL_ENABLED")?.as_deref() {
            None | Some("false") => {
                return Ok(Self {
                    notices: NoticeSettings {
                        from: DISABLED_SENDER.into(),
                        public_url: None,
                    },
                    email: None,
                })
            }
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
        CaseReportEmailConfiguration::new(from.clone(), public_url.clone())
            .map_err(|_| invalid())?;
        Ok(Self {
            notices: NoticeSettings {
                from,
                public_url: Some(public_url),
            },
            email: Some(api_key),
        })
    }
}

fn invalid() -> ApplicationError {
    ApplicationError::InvalidConfiguration("invalid contract email settings".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn load(values: &[(&str, &str)]) -> (Result<ContractSettings, ApplicationError>, Vec<String>) {
        let values: HashMap<_, _> = values.iter().copied().collect();
        let mut read = Vec::new();
        let result = ContractSettings::load_with(|name| {
            read.push(name.to_owned());
            Ok(values.get(name).map(|value| (*value).to_owned()))
        });
        (result, read)
    }

    const COMPLETE: [(&str, &str); 4] = [
        ("TT_CONTRACT_EMAIL_ENABLED", "true"),
        ("REPORT_EMAIL_FROM", "avisos@example.test"),
        ("REPORT_PUBLIC_URL", "https://qadra.example.test/"),
        ("RESEND_API_KEY", "re_test_key"),
    ];

    #[test]
    fn contract_email_is_off_by_default_without_reading_channel_secrets() {
        for values in [vec![], vec![("TT_CONTRACT_EMAIL_ENABLED", "false")]] {
            let (settings, read) = load(&values);
            let settings = settings.unwrap();
            assert!(settings.email.is_none());
            assert_eq!(settings.notices.from, DISABLED_SENDER);
            assert_eq!(settings.notices.public_url, None);
            assert_eq!(read, ["TT_CONTRACT_EMAIL_ENABLED"]);
        }
    }

    #[test]
    fn enabled_contract_email_reuses_the_report_sender_origin_and_secret() {
        let settings = load(&COMPLETE).0.unwrap();
        assert_eq!(settings.notices.from, "avisos@example.test");
        assert_eq!(
            settings.notices.public_url.as_deref(),
            Some("https://qadra.example.test/")
        );
        assert_eq!(
            settings.email.as_deref().map(String::as_str),
            Some("re_test_key")
        );
    }

    #[test]
    fn ambiguous_activation_or_an_incomplete_channel_is_refused_without_echoing_it() {
        let mut cases: Vec<Vec<(&str, &str)>> = vec![vec![("TT_CONTRACT_EMAIL_ENABLED", "yes")]];
        for missing in 1..COMPLETE.len() {
            let mut values = COMPLETE.to_vec();
            values.remove(missing);
            cases.push(values);
        }
        for (name, bad) in [
            ("REPORT_PUBLIC_URL", "http://qadra.example.test/"),
            ("REPORT_PUBLIC_URL", "https://qadra.example.test/inbox"),
            ("RESEND_API_KEY", "secret with spaces"),
            ("REPORT_EMAIL_FROM", "not an address"),
        ] {
            let mut values = COMPLETE.to_vec();
            values.retain(|(key, _)| *key != name);
            values.push((name, bad));
            cases.push(values);
        }
        for values in cases {
            let error = load(&values).0.err().expect("refused settings");
            let text = error.to_string();
            assert!(
                !text.contains("re_test_key") && !text.contains("spaces"),
                "{text}"
            );
        }
    }
}
