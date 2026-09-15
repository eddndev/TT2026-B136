use application::{documents::StageDocumentFormat, ApplicationError};
use rustix::event::{poll, PollFd, PollFlags, Timespec};
use rustix::fs::{fcntl_getfl, fcntl_setfl, OFlags};
use std::{
    io::{ErrorKind, Read, Write},
    os::unix::process::ExitStatusExt,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use super::super::worker::{decode, protocol_error, REQUEST_MAGIC, WORKER_FLAG};

pub(super) fn run(
    executable: &Path,
    library: &Path,
    inputs: &[&[u8]],
    timeout: Duration,
) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
    if !(1..=2).contains(&inputs.len()) {
        return Err(protocol_error());
    }
    if inputs.iter().any(|input| input.len() > 16 * 1024 * 1024) {
        return Err(ApplicationError::StageSupportTooLarge);
    }
    let mut header = REQUEST_MAGIC.to_vec();
    header.push(inputs.len() as u8);
    let lengths = inputs
        .iter()
        .map(|input| (input.len() as u32).to_be_bytes())
        .collect::<Vec<_>>();
    let mut chunks: Vec<&[u8]> = vec![&header];
    for (length, input) in lengths.iter().zip(inputs) {
        chunks.push(length);
        chunks.push(input);
    }
    let deadline = Instant::now() + timeout;
    let child = Command::new(executable)
        .arg(WORKER_FLAG)
        .arg(library)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| protocol_error())?;
    let mut guard = ChildGuard(child);
    let mut input = guard.0.stdin.take();
    let mut output = guard.0.stdout.take().ok_or_else(protocol_error)?;
    let stdin = input.as_ref().ok_or_else(protocol_error)?;
    fcntl_setfl(
        stdin,
        fcntl_getfl(stdin).map_err(|_| protocol_error())? | OFlags::NONBLOCK,
    )
    .map_err(|_| protocol_error())?;
    fcntl_setfl(
        &output,
        fcntl_getfl(&output).map_err(|_| protocol_error())? | OFlags::NONBLOCK,
    )
    .map_err(|_| protocol_error())?;
    let (mut chunk, mut offset) = (0usize, 0usize);
    let mut response = [0u8; 10];
    let mut length = 0;
    let mut eof = false;
    let mut status = None;
    loop {
        if Instant::now() >= deadline {
            return Err(ApplicationError::StageSupportValidationLimit);
        }
        if let Some(stdin) = input.as_mut() {
            match stdin.write(&chunks[chunk][offset..]) {
                Ok(0) => {
                    input.take();
                }
                Ok(written) => {
                    offset += written;
                    if offset == chunks[chunk].len() {
                        chunk += 1;
                        offset = 0;
                        if chunk == chunks.len() {
                            input.take();
                        }
                    }
                }
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {}
                Err(error) if error.kind() == ErrorKind::BrokenPipe => {
                    input.take();
                }
                Err(_) => return Err(protocol_error()),
            }
        }
        match output.read(&mut response[length..]) {
            Ok(0) => eof = true,
            Ok(read) => {
                length += read;
                if length == response.len() {
                    return Err(protocol_error());
                }
            }
            Err(error)
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {}
            Err(_) => return Err(protocol_error()),
        }
        if status.is_none() {
            status = guard.0.try_wait().map_err(|_| protocol_error())?;
        }
        if let Some(status) = status {
            input.take();
            if !status.success() {
                return Err(match status.signal() {
                    Some(9 | 24) => ApplicationError::StageSupportValidationLimit,
                    _ => protocol_error(),
                });
            }
            if eof {
                let formats = decode(&response[..length], inputs.len())?;
                if chunk != chunks.len() {
                    return Err(protocol_error());
                }
                return Ok(formats);
            }
        }
        let mut descriptors = Vec::with_capacity(2);
        if let Some(stdin) = &input {
            descriptors.push(PollFd::new(stdin, PollFlags::OUT));
        }
        if !eof {
            descriptors.push(PollFd::new(&output, PollFlags::IN));
        }
        let pause = Timespec {
            tv_sec: 0,
            tv_nsec: 5_000_000,
        };
        match poll(&mut descriptors, Some(&pause)) {
            Ok(_) | Err(rustix::io::Errno::INTR) => {}
            Err(_) => return Err(protocol_error()),
        }
    }
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
