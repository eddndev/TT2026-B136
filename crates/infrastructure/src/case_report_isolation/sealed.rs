use application::{
    case_reports::{CaseReportError, MAX_REPORT_SNAPSHOT_BYTES},
    ApplicationError,
};
use std::fs::File;

pub(super) fn input(bytes: &[u8]) -> Result<File, ApplicationError> {
    if bytes.len() > MAX_REPORT_SNAPSHOT_BYTES {
        return Err(CaseReportError::CapacityExceeded.into());
    }
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{fcntl_add_seals, fcntl_get_seals, memfd_create, MemfdFlags, SealFlags};
        use std::io::{Seek, Write};
        let fd = memfd_create(
            "case-report-capture",
            MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
        )
        .map_err(|_| CaseReportError::RenderUnavailable)?;
        let mut input = File::from(fd);
        input
            .write_all(bytes)
            .map_err(|_| CaseReportError::RenderUnavailable)?;
        let seals = SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL;
        fcntl_add_seals(&input, seals).map_err(|_| CaseReportError::RenderUnavailable)?;
        if fcntl_get_seals(&input).map_err(|_| CaseReportError::RenderUnavailable)? != seals {
            return Err(CaseReportError::RenderUnavailable.into());
        }
        input
            .rewind()
            .map_err(|_| CaseReportError::RenderUnavailable)?;
        Ok(input)
    }
    #[cfg(not(target_os = "linux"))]
    Err(CaseReportError::RenderUnavailable.into())
}
