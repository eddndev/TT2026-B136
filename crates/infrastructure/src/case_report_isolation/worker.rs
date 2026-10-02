use super::protocol::{self, Context};
use application::{case_reports::*, ApplicationError};
use domain::crypto::Sha256Digest;
use std::{
    ffi::OsStr,
    io::{Read, Write},
};
use zeroize::Zeroizing;

pub(super) const FLAG: &str = "--case-report-render-worker";

/// Handles the private renderer before any configuration or service initialization.
pub fn worker_entry() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next()?.as_os_str() != OsStr::new(FLAG) {
        return None;
    }
    #[cfg(target_os = "linux")]
    {
        let format = match (args.next(), args.next()) {
            (Some(value), None) if value == "pdf" => CaseReportFormat::Pdf,
            (Some(value), None) if value == "csv" => CaseReportFormat::Csv,
            _ => rustix::runtime::exit_group(125),
        };
        if limits().and_then(|()| close_inherited()).is_err() {
            rustix::runtime::exit_group(125);
        }
        let empty = Context {
            report_id: CaseReportId::from_uuid(uuid::Uuid::nil()),
            snapshot_digest: Sha256Digest::from_array([0; 32]),
            format,
        };
        let encoded = match render(format) {
            Ok((context, bytes)) => protocol::encode(context, Ok(bytes)),
            Err(error) => protocol::encode(empty, Err(error)),
        };
        let response = match encoded {
            Ok(response) => Zeroizing::new(response),
            Err(error) => match protocol::encode(empty, Err(error)) {
                Ok(response) => Zeroizing::new(response),
                Err(_) => rustix::runtime::exit_group(125),
            },
        };
        let mut output = std::io::stdout().lock();
        let success = output
            .write_all(&response)
            .and_then(|()| output.flush())
            .is_ok();
        drop(output);
        drop(response);
        rustix::runtime::exit_group(if success { 0 } else { 125 });
    }
    #[cfg(not(target_os = "linux"))]
    Some(125)
}

fn render(format: CaseReportFormat) -> Result<(Context, Vec<u8>), ApplicationError> {
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .take((MAX_REPORT_SNAPSHOT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| CaseReportError::RenderUnavailable)?;
    if bytes.len() > MAX_REPORT_SNAPSHOT_BYTES {
        return Err(CaseReportError::CapacityExceeded.into());
    }
    let snapshot = crate::case_reports::codec::read_snapshot(&bytes)?;
    validate_case_report_snapshot(&crate::RingSha256Hasher, &snapshot)?;
    let context = Context {
        report_id: snapshot.report_id,
        snapshot_digest: snapshot.digest,
        format,
    };
    let renderer = crate::case_report_rendering::BoundedCaseReportRenderer::bundled()?;
    Ok((context, renderer.render(&snapshot, format)?))
}

#[cfg(target_os = "linux")]
fn limits() -> Result<(), ApplicationError> {
    use rustix::process::{setrlimit, Resource, Rlimit};
    for (resource, value) in [
        (Resource::Cpu, 15),
        (Resource::As, 512 * 1024 * 1024),
        (Resource::Core, 0),
        (Resource::Fsize, 0),
    ] {
        setrlimit(
            resource,
            Rlimit {
                current: Some(value),
                maximum: Some(value),
            },
        )
        .map_err(|_| CaseReportError::RenderUnavailable)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn close_inherited() -> Result<(), ApplicationError> {
    use rustix::fs::{open, Dir, Mode, OFlags};
    use std::os::fd::AsRawFd;
    let unavailable = |_| CaseReportError::RenderUnavailable;
    let directory = open(
        "/proc/self/fd",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(unavailable)?;
    let own_descriptor = directory.as_raw_fd();
    let mut entries = Dir::new(directory).map_err(unavailable)?;
    let mut descriptors = Vec::new();
    while let Some(entry) = entries.read() {
        let entry = entry.map_err(unavailable)?;
        let name = entry
            .file_name()
            .to_str()
            .map_err(|_| CaseReportError::RenderUnavailable)?;
        if matches!(name, "." | "..") {
            continue;
        }
        let descriptor = name
            .parse::<i32>()
            .map_err(|_| CaseReportError::RenderUnavailable)?;
        if descriptor > 2 && descriptor != own_descriptor {
            descriptors.push(descriptor);
        }
    }
    drop(entries);
    // This private entry has no threads or retained files. Only still-live
    // inherited descriptors were collected, excluding the iterator's own fd.
    for descriptor in descriptors {
        unsafe {
            rustix::io::close(descriptor);
        }
    }
    Ok(())
}
