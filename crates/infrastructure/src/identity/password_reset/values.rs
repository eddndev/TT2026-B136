use application::ApplicationError;
use domain::clock::OffsetDateTime;
use postgres::{types::FromSql, Row};

pub(super) fn value<'a, T: FromSql<'a>>(row: &'a Row, name: &str) -> Result<T, ApplicationError> {
    row.try_get(name).map_err(|_| invalid_state())
}

pub(super) fn nonnegative(row: &Row, name: &str) -> Result<u64, ApplicationError> {
    let number: i64 = value(row, name)?;
    u64::try_from(number).map_err(|_| invalid_state())
}

pub(super) fn timestamp(row: &Row, name: &str) -> Result<OffsetDateTime, ApplicationError> {
    let micros: i64 = value(row, name)?;
    let nanos = i128::from(micros)
        .checked_mul(1000)
        .ok_or_else(invalid_state)?;
    let at = OffsetDateTime::from_unix_timestamp_nanos(nanos).map_err(|_| invalid_state())?;
    if !(1..=9999).contains(&at.year()) {
        return Err(invalid_state());
    }
    Ok(at)
}

pub(super) fn invalid_state() -> ApplicationError {
    ApplicationError::Port("password reset storage returned inconsistent state".into())
}

pub(super) fn storage_error() -> ApplicationError {
    // PostgreSQL DETAIL can contain token digests and private account values.
    ApplicationError::Port("password reset storage operation failed".into())
}
