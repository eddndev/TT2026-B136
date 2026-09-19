//! Shared authentication of durable worker jobs and technical revision linkage.
//! Source histories are verified by their existing exact readers. This module
//! never loads a deadline through its full reader, avoiding recursive decoding.
mod event;
mod job;
mod record;

pub(crate) use job::load_job;
pub(crate) use record::validate_technical_record;

use application::{
    deadline_reevaluation::TechnicalCause,
    deadlines::{DeadlineError, DeadlineId, DeadlineOperationId},
    ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, typed_participants::Uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedDeadlineJob {
    pub id: Uuid,
    pub operation_id: DeadlineOperationId,
    pub deadline_id: DeadlineId,
    pub case_id: CaseId,
    pub cause: TechnicalCause,
    pub created_at: OffsetDateTime,
}

fn timestamp(seconds: i64, nanos: i32) -> Result<OffsetDateTime, ApplicationError> {
    let at = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(inconsistent)?
        .replace_nanosecond(u32::try_from(nanos).map_err(inconsistent)?)
        .map_err(inconsistent)?;
    if !(1..=9999).contains(&at.year()) {
        return Err(inconsistent("worker timestamp year is outside bounds"));
    }
    Ok(at)
}

fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    DeadlineError::StoredInconsistent(error.to_string()).into()
}

fn source_error(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::Port(_) | ApplicationError::ClassifiedPort { .. } => error,
        _ => inconsistent(error),
    }
}

fn port(error: postgres::Error) -> ApplicationError {
    crate::postgres_port::error("deadline worker provenance database", error)
}
