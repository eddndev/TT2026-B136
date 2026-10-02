//! Durable report requests, captures and paired exports in audited transactions.
mod access;
mod capture;
pub(crate) mod codec;
mod completion;
mod payload;
mod reads;
mod requests;
mod storage;
mod worker;
use application::{case_reports::*, identity::Principal, ApplicationError};
use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::{DocumentHasher, Sha256Digest},
};
use postgres::{Client, Transaction};
use std::sync::{Arc, Mutex};
use time::{format_description::well_known::Rfc3339, UtcOffset};
use uuid::Uuid;

pub struct PostgresCaseReportStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
    protector: Arc<dyn CaseReportProtector>,
}
impl PostgresCaseReportStore {
    pub fn open(
        url: &(impl crate::PostgresConnectionSource + ?Sized),
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
        protector: Arc<dyn CaseReportProtector>,
    ) -> Result<Self, ApplicationError> {
        let mut client = crate::postgres::open(url)?;
        client
            .batch_execute("SET lock_timeout='2s'; SET statement_timeout='15s'")
            .map_err(port)?;
        Ok(Self {
            client: Mutex::new(client),
            hasher,
            clock,
            protector,
        })
    }
    fn observed(&self, lower: OffsetDateTime) -> Result<OffsetDateTime, ApplicationError> {
        let now = self.clock.now().to_offset(UtcOffset::UTC);
        if lower.offset() != UtcOffset::UTC || !(1..=9999).contains(&now.year()) || now < lower {
            return Err(inconsistent("report clock predates its operation"));
        }
        Ok(now)
    }
    fn tx<T>(
        &self,
        action: impl FnOnce(&mut Transaction<'_>) -> Result<T, ApplicationError>,
    ) -> Result<T, ApplicationError> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| inconsistent("report database mutex poisoned"))?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let value = action(&mut tx)?;
        tx.commit().map_err(port)?;
        Ok(value)
    }
}
fn port(error: postgres::Error) -> ApplicationError {
    crate::postgres_port::error("case report database", error)
}
fn inconsistent(message: impl std::fmt::Display) -> ApplicationError {
    CaseReportError::StoredInconsistent(message.to_string()).into()
}
fn timestamp(at: OffsetDateTime) -> Result<String, ApplicationError> {
    at.format(&Rfc3339).map_err(inconsistent)
}
fn parse_time(value: &str) -> Result<OffsetDateTime, ApplicationError> {
    let at = OffsetDateTime::parse(value, &Rfc3339).map_err(inconsistent)?;
    if at.offset() != UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
        return Err(inconsistent("report time is not UTC"));
    }
    Ok(at)
}
fn digest(value: &[u8]) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(value).map_err(inconsistent)
}
fn audit(
    tx: &mut Transaction<'_>,
    report: &CaseReportDetail,
    action: &str,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    crate::audit_postgres::append_transaction(
        tx,
        &report.requester.principal.email,
        action,
        &format!(
            "case-report:{}:request:{}",
            report.id, report.request_digest
        ),
        at,
    )?;
    Ok(())
}
fn scope_name(value: CaseReportScope) -> &'static str {
    match value {
        CaseReportScope::Office => "office",
        CaseReportScope::AssignedCases => "assigned_cases",
    }
}
fn format_name(value: CaseReportFormat) -> &'static str {
    match value {
        CaseReportFormat::Pdf => "pdf",
        CaseReportFormat::Csv => "csv",
    }
}
fn format_parse(value: &str) -> Result<CaseReportFormat, ApplicationError> {
    match value {
        "pdf" => Ok(CaseReportFormat::Pdf),
        "csv" => Ok(CaseReportFormat::Csv),
        _ => Err(inconsistent("unknown report artifact format")),
    }
}
