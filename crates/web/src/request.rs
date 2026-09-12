//! Shared request metadata parsing for every authenticated route.

use application::ApplicationError;
use axum::http::{header::AUTHORIZATION, HeaderMap};

use crate::error::ApiError;

pub(crate) fn bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let value = values.next().and_then(|value| value.to_str().ok());
    if values.next().is_some() {
        return Err(ApplicationError::InvalidSession.into());
    }
    value
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, token)| {
            scheme.eq_ignore_ascii_case("bearer")
                && !token.is_empty()
                && token.bytes().all(|byte| byte.is_ascii_graphic())
        })
        .map(|(_, token)| token.to_owned())
        .ok_or(ApplicationError::InvalidSession.into())
}
