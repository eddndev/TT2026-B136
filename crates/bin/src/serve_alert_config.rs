//! Optional all-or-none email configuration, with secrets excluded from Debug.

use application::{alerts::AlertEmailConfiguration, ApplicationError};
use zeroize::Zeroizing;

pub(crate) struct AlertEmailSettings {
    configuration: AlertEmailConfiguration,
    api_key: Zeroizing<String>,
}
impl AlertEmailSettings {
    pub(crate) fn from_optional(
        api_key: Option<String>,
        from_email: Option<String>,
        login_url: Option<String>,
    ) -> Result<Option<Self>, ApplicationError> {
        let api_key = api_key.map(Zeroizing::new);
        match (api_key, from_email, login_url) {
            (None, None, None) => Ok(None),
            (Some(api_key), Some(from_email), Some(login_url)) => {
                if api_key.is_empty()
                    || api_key.len() > 1024
                    || !api_key.bytes().all(|byte| byte.is_ascii_graphic())
                {
                    return Err(ApplicationError::InvalidConfiguration(
                        "invalid alert email API key".into(),
                    ));
                }
                let configuration = AlertEmailConfiguration {
                    from_email,
                    login_url,
                };
                infrastructure::alert_email::validate_configuration(&configuration)?;
                Ok(Some(Self {
                    configuration,
                    api_key,
                }))
            }
            _ => Err(ApplicationError::InvalidConfiguration(
                "alert email requires API key, sender and login URL together".into(),
            )),
        }
    }
    pub(crate) fn into_parts(self) -> (AlertEmailConfiguration, Zeroizing<String>) {
        (self.configuration, self.api_key)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_absent_disables_email_and_complete_configuration_preserves_exact_values() {
        assert!(AlertEmailSettings::from_optional(None, None, None)
            .unwrap()
            .is_none());
        let settings = AlertEmailSettings::from_optional(
            Some("private-test-key".into()),
            Some("qadra@example.test".into()),
            Some("https://qadra.example.test/login".into()),
        )
        .unwrap()
        .expect("complete email settings");
        let (configuration, key) = settings.into_parts();
        assert_eq!(configuration.from_email, "qadra@example.test");
        assert_eq!(configuration.login_url, "https://qadra.example.test/login");
        assert_eq!(key.as_str(), "private-test-key");
    }
    #[test]
    fn partial_empty_or_invalid_values_are_rejected_without_disclosing_secret_content() {
        for present in 1..7 {
            let result = AlertEmailSettings::from_optional(
                (present & 1 != 0).then(|| "private-test-key".into()),
                (present & 2 != 0).then(|| "qadra@example.test".into()),
                (present & 4 != 0).then(|| "https://qadra.example.test/login".into()),
            );
            let Err(error) = result else {
                panic!("partial settings accepted")
            };
            assert!(matches!(error, ApplicationError::InvalidConfiguration(_)));
            assert!(!error.to_string().contains("private-test-key"));
        }
        for (key, from, url) in [
            ("", "qadra@example.test", "https://qadra.example.test/login"),
            (
                "private-test-key",
                "invalid",
                "https://qadra.example.test/login",
            ),
            (
                "private-test-key",
                "qadra@example.test",
                "https://qadra.example.test/cases/private",
            ),
        ] {
            assert!(AlertEmailSettings::from_optional(
                Some(key.into()),
                Some(from.into()),
                Some(url.into())
            )
            .is_err());
        }
    }
}
