use super::{Error, Result};

/// Counters shared by every DOCX admitted in one worker batch.
pub(crate) struct DocxBudget {
    remaining_bytes: usize,
    remaining_events: usize,
}
impl DocxBudget {
    pub(crate) fn standard() -> Self {
        Self {
            remaining_bytes: 64 * 1024 * 1024,
            remaining_events: 1_000_000,
        }
    }
    pub(super) fn bytes(&mut self, count: usize) -> Result<()> {
        self.remaining_bytes = self
            .remaining_bytes
            .checked_sub(count)
            .ok_or(Error::StageSupportValidationLimit)?;
        Ok(())
    }
    pub(super) fn event(&mut self) -> Result<()> {
        self.remaining_events = self
            .remaining_events
            .checked_sub(1)
            .ok_or(Error::StageSupportValidationLimit)?;
        Ok(())
    }
}
