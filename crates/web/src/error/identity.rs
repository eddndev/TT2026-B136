//! HTTP mapping for account identity input errors:
//! docs/adr/0087-account-display-names.md.

use super::ApiError;
use application::ApplicationError;
use axum::http::StatusCode;
use domain::DomainError;

pub(super) fn map(error: ApplicationError) -> Result<ApiError, ApplicationError> {
    match error {
        ApplicationError::Domain(DomainError::InvalidDisplayName(reason)) => Ok(ApiError {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "invalid_display_name",
            message: format!("invalid display name: {reason}"),
        }),
        other => Err(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_invalid_display_name_is_unprocessable_with_its_reason() {
        let error = map(ApplicationError::Domain(DomainError::InvalidDisplayName(
            "length",
        )))
        .unwrap_or_else(|_| panic!("display name error was not mapped"));
        assert_eq!(error.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error.code, "invalid_display_name");
        assert_eq!(error.message, "invalid display name: length");
        assert!(map(ApplicationError::InvalidSession).is_err());
    }
}
