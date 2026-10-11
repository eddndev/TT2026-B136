//! Initial request-to-availability estimate, including the durable queue wait.
use crate::ApplicationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaseReportEstimate {
    total_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaseReportTimingSample {
    pub cases: u32,
    pub service_ms: u64,
}

/// Median measured service cost at comparable volume. Three observations within
/// 25 percent of the requested case count are required; cold or substantially
/// different workloads remain unknown instead of extrapolating a runtime rate.
pub fn estimate_report_service_ms(cases: u32, samples: &[CaseReportTimingSample]) -> Option<u64> {
    let volume = u128::from(cases) + 1;
    let mut measured: Vec<u64> = samples
        .iter()
        .filter_map(|sample| {
            let previous = u128::from(sample.cases) + 1;
            if sample.service_ms == 0 || previous.abs_diff(volume) * 4 > volume {
                return None;
            }
            u64::try_from((u128::from(sample.service_ms) * volume).div_ceil(previous)).ok()
        })
        .collect();
    if measured.len() < 3 {
        return None;
    }
    measured.sort_unstable();
    Some(measured[measured.len() / 2])
}

impl CaseReportEstimate {
    pub fn new(total_ms: u64) -> Result<Self, ApplicationError> {
        if total_ms == 0 || total_ms > i64::MAX as u64 {
            return Err(ApplicationError::InvalidInput(
                "report estimate must be a positive stored duration".into(),
            ));
        }
        Ok(Self { total_ms })
    }

    pub fn total_ms(self) -> u64 {
        self.total_ms
    }

    pub fn requires_ready_email(self) -> bool {
        self.total_ms > 2_000
    }
}
