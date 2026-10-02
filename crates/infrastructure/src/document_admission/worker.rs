use application::{
    documents::{AdmittedDocumentFormat as Format, DocumentUploadError, MAX_DOCUMENT_UPLOAD_BYTES},
    ApplicationError,
};
use std::{
    ffi::OsStr,
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;

pub(super) const INSPECT_FLAG: &str = "--document-admission-worker";
pub(super) const EXEC_FLAG: &str = "--document-decoder-worker";
const MAGIC: &[u8; 5] = b"GADM1";

/// Handles private, bounded parser invocations before server initialization.
pub fn worker_entry() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    let first = args.next()?;
    if first != OsStr::new(INSPECT_FLAG) && first != OsStr::new(EXEC_FLAG) {
        return None;
    }
    #[cfg(target_os = "linux")]
    {
        if first == OsStr::new(EXEC_FLAG) {
            use std::os::unix::process::CommandExt;
            let cpu = match args.next().as_deref() {
                Some(v) if v == "probe" => 2,
                Some(v) if v == "decode" => 8,
                _ => rustix::runtime::exit_group(125),
            };
            let program = match args.next() {
                Some(v) if Path::new(&v).is_absolute() => v,
                _ => rustix::runtime::exit_group(125),
            };
            if limits(cpu, 512 * 1024 * 1024)
                .and_then(|()| close_inherited())
                .is_err()
            {
                rustix::runtime::exit_group(125);
            }
            let _error = std::process::Command::new(program)
                .args(args)
                .env_clear()
                // Bound native allocator reservations; see docs/adr/0058-bounded-general-document-admission.md.
                .env("MALLOC_ARENA_MAX", "2")
                .exec();
            rustix::runtime::exit_group(125);
        }
        let result = match (args.next(), args.next()) {
            (Some(library), None) => limits(5, 256 * 1024 * 1024)
                .and_then(|()| close_inherited())
                .and_then(|()| inspect(Path::new(&library))),
            _ => Err(DocumentUploadError::Unavailable.into()),
        };
        let response = encode(result);
        let mut output = std::io::stdout().lock();
        let success = output
            .write_all(&response)
            .and_then(|()| output.flush())
            .is_ok();
        drop(output);
        rustix::runtime::exit_group(if success { 0 } else { 125 });
    }
    #[cfg(not(target_os = "linux"))]
    Some(125)
}

#[cfg(target_os = "linux")]
fn limits(cpu: u64, address_space: u64) -> Result<(), ApplicationError> {
    use rustix::process::{setrlimit, Resource, Rlimit};
    for (resource, value) in [
        (Resource::Cpu, cpu),
        (Resource::As, address_space),
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
        .map_err(|_| DocumentUploadError::Unavailable)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn close_inherited() -> Result<(), ApplicationError> {
    use rustix::fs::{open, Dir, Mode, OFlags};
    use std::os::fd::AsRawFd;
    let unavailable = |_| DocumentUploadError::Unavailable;
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
            .map_err(|_| DocumentUploadError::Unavailable)?;
        if matches!(name, "." | "..") {
            continue;
        }
        let fd = name
            .parse::<i32>()
            .map_err(|_| DocumentUploadError::Unavailable)?;
        if fd > 2 && fd != own_descriptor {
            descriptors.push(fd);
        }
    }
    drop(entries);
    // This private process has not started threads or retained other open files.
    // Each inherited descriptor is still valid; the iterator's own fd is excluded.
    for fd in descriptors {
        unsafe {
            rustix::io::close(fd);
        }
    }
    Ok(())
}

fn inspect(library: &Path) -> Result<Format, ApplicationError> {
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .take((MAX_DOCUMENT_UPLOAD_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| DocumentUploadError::Unavailable)?;
    let format = super::structure::inspect(&bytes)?;
    if matches!(format, Format::Pdf | Format::Docx) {
        crate::document_formats::validate_upload(&bytes, library).map_err(|error| match error {
            ApplicationError::StageSupportFormatRejected => DocumentUploadError::Invalid,
            ApplicationError::StageSupportTooLarge
            | ApplicationError::StageSupportValidationLimit => DocumentUploadError::Limit,
            _ => DocumentUploadError::Unavailable,
        })?;
    }
    Ok(format)
}

fn encode(result: Result<Format, ApplicationError>) -> [u8; 7] {
    let mut bytes = [0; 7];
    bytes[..5].copy_from_slice(MAGIC);
    match result {
        Ok(format) => {
            bytes[6] = match format {
                Format::Pdf => 0,
                Format::Docx => 1,
                Format::Txt => 2,
                Format::Jpeg => 3,
                Format::Png => 4,
                Format::Mp3 => 5,
                Format::Wav => 6,
                Format::Mp4 => 7,
            }
        }
        Err(error) => {
            bytes[5] = match error {
                ApplicationError::DocumentUpload(DocumentUploadError::Unsupported) => 1,
                ApplicationError::DocumentUpload(DocumentUploadError::Invalid) => 2,
                ApplicationError::DocumentUpload(DocumentUploadError::Limit) => 3,
                _ => 4,
            }
        }
    }
    bytes
}

pub(super) fn decode(bytes: &[u8]) -> Result<Format, ApplicationError> {
    if bytes.len() != 7 || &bytes[..5] != MAGIC {
        return Err(DocumentUploadError::Unavailable.into());
    }
    if bytes[5] != 0 {
        if bytes[6] != 0 {
            return Err(DocumentUploadError::Unavailable.into());
        }
        return Err(match bytes[5] {
            1 => DocumentUploadError::Unsupported,
            2 => DocumentUploadError::Invalid,
            3 => DocumentUploadError::Limit,
            _ => DocumentUploadError::Unavailable,
        }
        .into());
    }
    Ok(match bytes[6] {
        0 => Format::Pdf,
        1 => Format::Docx,
        2 => Format::Txt,
        3 => Format::Jpeg,
        4 => Format::Png,
        5 => Format::Mp3,
        6 => Format::Wav,
        7 => Format::Mp4,
        _ => return Err(DocumentUploadError::Unavailable.into()),
    })
}
