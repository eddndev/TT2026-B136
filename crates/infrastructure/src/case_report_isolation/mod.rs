//! Process boundary for rendering one captured private report.
#[cfg(target_os = "linux")]
mod process;
mod protocol;
mod sealed;
#[cfg(all(test, target_os = "linux"))]
mod tests;
mod worker;

use application::{case_reports::*, ApplicationError};
use std::path::PathBuf;
pub use worker::worker_entry;

/// Executes only the configured trusted binary's private report entry point.
pub struct IsolatedCaseReportRenderer {
    executable: PathBuf,
}
impl IsolatedCaseReportRenderer {
    pub fn new(executable: PathBuf) -> Result<Self, ApplicationError> {
        if !executable.is_absolute() || !executable.is_file() {
            return Err(CaseReportError::RenderUnavailable.into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = executable
                .metadata()
                .map_err(|_| CaseReportError::RenderUnavailable)?;
            if metadata.permissions().mode() & 0o111 == 0 {
                return Err(CaseReportError::RenderUnavailable.into());
            }
        }
        Ok(Self { executable })
    }
}
impl CaseReportRenderer for IsolatedCaseReportRenderer {
    fn render(
        &self,
        snapshot: &CaseReportSnapshot,
        format: CaseReportFormat,
    ) -> Result<Vec<u8>, ApplicationError> {
        #[cfg(target_os = "linux")]
        {
            use std::time::{Duration, Instant};
            use zeroize::Zeroizing;
            let deadline = Instant::now() + Duration::from_secs(20);
            validate_case_report_snapshot(&crate::RingSha256Hasher, snapshot)?;
            let bytes = Zeroizing::new(crate::case_reports::codec::snapshot(snapshot)?);
            let input = sealed::input(&bytes)?;
            let format_name = match format {
                CaseReportFormat::Pdf => "pdf",
                CaseReportFormat::Csv => "csv",
            };
            let output = Zeroizing::new(process::run(
                &self.executable,
                &[worker::FLAG.into(), format_name.into()],
                &input,
                deadline,
                MAX_REPORT_ARTIFACT_BYTES + protocol::HEADER_LEN,
            )?);
            protocol::decode(
                &output,
                protocol::Context {
                    report_id: snapshot.report_id,
                    snapshot_digest: snapshot.digest,
                    format,
                },
            )
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (&self.executable, snapshot, format);
            Err(CaseReportError::RenderUnavailable.into())
        }
    }
}
