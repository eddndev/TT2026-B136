use application::{documents::StageDocumentFormat, ApplicationError};
use std::{
    ffi::OsStr,
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;

use super::{
    docx::{validate_docx, DocxBudget},
    PdfLibrary,
};

pub(super) const REQUEST_MAGIC: &[u8; 5] = b"DFMT1";
pub(super) const RESPONSE_MAGIC: &[u8; 5] = b"DFMR1";
pub(super) const WORKER_FLAG: &str = "--document-format-worker";

/// Handles the private worker invocation before configuration or logging loads.
/// Normal invocations return None. Linux workers flush and exit internally;
/// unsupported platforms return an error code for the caller to exit with.
pub fn worker_entry() -> Option<i32> {
    let code = {
        let mut args = std::env::args_os().skip(1);
        if args.next().as_deref() != Some(OsStr::new(WORKER_FLAG)) {
            return None;
        }
        let result = match (args.next(), args.next()) {
            (Some(library), None) => limits().and_then(|()| {
                let input = std::io::stdin();
                process(&mut input.lock(), Path::new(&library))
            }),
            _ => Err(protocol_error()),
        };
        let response = encode(result);
        let mut output = std::io::stdout().lock();
        if output
            .write_all(&response)
            .and_then(|()| output.flush())
            .is_ok()
        {
            0
        } else {
            1
        }
    };
    // Parsing sessions, plaintext, response buffers and I/O locks have dropped.
    // Immediate exit preserves file limits during shutdown too: native atexit
    // handlers and instrumentation writers must not run after the response.
    #[cfg(target_os = "linux")]
    rustix::runtime::exit_group(code);
    #[cfg(not(target_os = "linux"))]
    Some(code.max(1))
}

#[cfg(target_os = "linux")]
fn limits() -> Result<(), ApplicationError> {
    use rustix::process::{setrlimit, Resource, Rlimit};
    for (resource, value) in [
        (Resource::Core, 0),
        (Resource::Fsize, 0),
        (Resource::Cpu, 5),
        (Resource::As, 256 * 1024 * 1024),
    ] {
        setrlimit(
            resource,
            Rlimit {
                current: Some(value),
                maximum: Some(value),
            },
        )
        .map_err(|_| {
            ApplicationError::InvalidConfiguration(
                "document worker resource limits are unavailable".into(),
            )
        })?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn limits() -> Result<(), ApplicationError> {
    Err(ApplicationError::InvalidConfiguration(
        "isolated document format validation requires Linux".into(),
    ))
}

fn process(
    input: &mut impl Read,
    library_path: &Path,
) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
    let mut header = [0u8; 6];
    input
        .read_exact(&mut header)
        .map_err(|_| protocol_error())?;
    if &header[..5] != REQUEST_MAGIC || !(1..=2).contains(&header[5]) {
        return Err(protocol_error());
    }
    let mut budget = DocxBudget::standard();
    let mut pdf = None;
    let mut total = 0usize;
    let mut formats = Vec::with_capacity(usize::from(header[5]));
    for _ in 0..header[5] {
        let mut length = [0u8; 4];
        input
            .read_exact(&mut length)
            .map_err(|_| protocol_error())?;
        let length = u32::from_be_bytes(length) as usize;
        total = total
            .checked_add(length)
            .ok_or(ApplicationError::StageSupportTooLarge)?;
        if length > 16 * 1024 * 1024 || total > 32 * 1024 * 1024 {
            return Err(ApplicationError::StageSupportTooLarge);
        }
        let mut bytes = Zeroizing::new(Vec::new());
        bytes
            .try_reserve_exact(length)
            .map_err(|_| ApplicationError::StageSupportValidationLimit)?;
        bytes.resize(length, 0);
        input.read_exact(&mut bytes).map_err(|_| protocol_error())?;
        if bytes.starts_with(b"%PDF-") {
            if pdf.is_none() {
                pdf = Some(PdfLibrary::open(library_path)?);
            }
            pdf.as_ref().ok_or_else(protocol_error)?.validate(&bytes)?;
            formats.push(StageDocumentFormat::Pdf);
        } else if bytes.starts_with(b"PK\x03\x04") {
            validate_docx(&bytes, &mut budget)?;
            formats.push(StageDocumentFormat::Docx);
        } else {
            return Err(ApplicationError::StageSupportFormatRejected);
        }
    }
    let mut trailing = [0u8; 1];
    if input.read(&mut trailing).map_err(|_| protocol_error())? != 0 {
        return Err(protocol_error());
    }
    Ok(formats)
}

fn encode(result: Result<Vec<StageDocumentFormat>, ApplicationError>) -> Vec<u8> {
    let mut out = RESPONSE_MAGIC.to_vec();
    match result {
        Ok(formats) => {
            out.extend_from_slice(&[0, formats.len() as u8]);
            out.extend(formats.into_iter().map(|value| match value {
                StageDocumentFormat::Pdf => 0,
                StageDocumentFormat::Docx => 1,
            }));
        }
        Err(error) => out.extend_from_slice(&[
            match error {
                ApplicationError::StageSupportFormatRejected => 1,
                ApplicationError::StageSupportTooLarge => 2,
                ApplicationError::StageSupportValidationLimit => 3,
                _ => 4,
            },
            0,
        ]),
    }
    out
}

pub(super) fn decode(
    bytes: &[u8],
    expected: usize,
) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
    if bytes.len() < 7 || &bytes[..5] != RESPONSE_MAGIC {
        return Err(protocol_error());
    }
    if bytes[5] != 0 {
        if bytes.len() != 7 || bytes[6] != 0 {
            return Err(protocol_error());
        }
        return Err(match bytes[5] {
            1 => ApplicationError::StageSupportFormatRejected,
            2 => ApplicationError::StageSupportTooLarge,
            3 => ApplicationError::StageSupportValidationLimit,
            _ => protocol_error(),
        });
    }
    if usize::from(bytes[6]) != expected || bytes.len() != 7 + expected {
        return Err(protocol_error());
    }
    bytes[7..]
        .iter()
        .map(|value| match value {
            0 => Ok(StageDocumentFormat::Pdf),
            1 => Ok(StageDocumentFormat::Docx),
            _ => Err(protocol_error()),
        })
        .collect()
}

pub(super) fn protocol_error() -> ApplicationError {
    ApplicationError::Port("document format worker failed its protocol".into())
}
