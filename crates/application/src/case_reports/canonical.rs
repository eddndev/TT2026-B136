use super::*;
use crate::{cases::CaseStatusFilter, identity::Principal, ApplicationError};
use domain::{clock::OffsetDateTime, crypto::Sha256Digest, typed_participants::Uuid};

/// Length-prefixed bytes and fixed-width integers form an unambiguous versioned
/// encoding. Counts precede sequences; optional values always include a tag.
pub(super) struct Encoder(Vec<u8>);
impl Encoder {
    pub(super) fn new(domain: &[u8]) -> Result<Self, ApplicationError> {
        let mut value = Self(Vec::new());
        value.bytes(domain)?;
        value.number(1)?;
        Ok(value)
    }
    fn append(&mut self, bytes: &[u8]) -> Result<(), ApplicationError> {
        if bytes.len() > MAX_REPORT_SNAPSHOT_BYTES.saturating_sub(self.0.len()) {
            return Err(CaseReportError::CapacityExceeded.into());
        }
        self.0.extend_from_slice(bytes);
        Ok(())
    }
    pub(super) fn bytes(&mut self, bytes: &[u8]) -> Result<(), ApplicationError> {
        self.number(bytes.len() as u64)?;
        self.append(bytes)
    }
    pub(super) fn number(&mut self, value: u64) -> Result<(), ApplicationError> {
        self.append(&value.to_be_bytes())
    }
    pub(super) fn uuid(&mut self, value: Uuid) -> Result<(), ApplicationError> {
        self.append(value.as_bytes())
    }
    pub(super) fn time(&mut self, value: OffsetDateTime) -> Result<(), ApplicationError> {
        self.append(&value.unix_timestamp_nanos().to_be_bytes())
    }
    pub(super) fn digest(&mut self, value: Sha256Digest) -> Result<(), ApplicationError> {
        self.append(value.as_bytes())
    }
    pub(super) fn principal(&mut self, value: &Principal) -> Result<(), ApplicationError> {
        self.uuid(value.id.as_uuid())?;
        self.bytes(value.email.as_bytes())?;
        self.bytes(value.role.as_str().as_bytes())
    }
    pub(super) fn requester(
        &mut self,
        value: &CaseReportRequester,
    ) -> Result<(), ApplicationError> {
        self.principal(&value.principal)?;
        self.number(value.account_revision)?;
        self.number(value.auth_generation)
    }
    pub(super) fn scope(&mut self, value: CaseReportScope) -> Result<(), ApplicationError> {
        self.number(match value {
            CaseReportScope::Office => 1,
            CaseReportScope::AssignedCases => 2,
        })
    }
    pub(super) fn filters(&mut self, value: &CaseReportFilters) -> Result<(), ApplicationError> {
        self.time(value.period_from)?;
        self.time(value.period_before)?;
        self.number(match value.status {
            CaseStatusFilter::All => 0,
            CaseStatusFilter::Active => 1,
            CaseStatusFilter::Closed => 2,
        })?;
        self.number(u64::from(value.litigator.is_some()))?;
        if let Some(id) = value.litigator {
            self.uuid(id.as_uuid())?;
        }
        Ok(())
    }
    pub(super) fn litigator(
        &mut self,
        value: &CaseReportLitigator,
    ) -> Result<(), ApplicationError> {
        self.uuid(value.user_id.as_uuid())?;
        self.bytes(value.email.as_bytes())
    }
    pub(super) fn finish(self) -> Vec<u8> {
        self.0
    }
}
