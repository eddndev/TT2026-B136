use application::{case_reports::CaseReportError, ApplicationError};
use rustix::{
    event::{poll, PollFd, PollFlags, Timespec},
    fs::{fcntl_getfl, fcntl_setfl, OFlags},
    process::{kill_process_group, waitid, Pid, Signal, WaitId, WaitIdOptions},
};
use std::{
    ffi::OsString,
    fs::File,
    io::{ErrorKind, Read},
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

fn unavailable() -> ApplicationError {
    CaseReportError::RenderUnavailable.into()
}
fn limit() -> ApplicationError {
    CaseReportError::CapacityExceeded.into()
}

/// Own one report renderer process group until its bounded output and exit are settled.
/// The caller rewinds the sealed capture and supplies one bounded render deadline.
pub(super) fn run(
    executable: &Path,
    args: &[OsString],
    input: &File,
    deadline: Instant,
    output_limit: usize,
) -> Result<Vec<u8>, ApplicationError> {
    if Instant::now() >= deadline {
        return Err(limit());
    }
    let child = Command::new(executable)
        .args(args)
        .env_clear()
        .current_dir("/")
        .process_group(0)
        .stdin(Stdio::from(input.try_clone().map_err(|_| unavailable())?))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| unavailable())?;
    let mut child = OwnedGroup::new(child);
    let mut stdout = child.child.stdout.take().ok_or_else(unavailable)?;
    fcntl_setfl(
        &stdout,
        fcntl_getfl(&stdout).map_err(|_| unavailable())? | OFlags::NONBLOCK,
    )
    .map_err(|_| unavailable())?;
    let mut output = Vec::new();
    let mut buffer = [0; 4096];
    let mut eof = false;
    let mut exited = false;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err(limit());
        }
        if !exited {
            let observed = waitid(
                WaitId::Pid(child.pid),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
            );
            match observed {
                Ok(Some(status)) => {
                    // Keep the leader unreaped until its owned group is killed;
                    // this prevents signaling a reused process-group identifier.
                    child.terminate()?;
                    if status.exit_status() == Some(125) {
                        return Err(unavailable());
                    }
                    if status.exit_status() != Some(0) {
                        return Err(match status.terminating_signal() {
                            Some(9 | 24) => limit(),
                            _ => CaseReportError::RenderFailed.into(),
                        });
                    }
                    exited = true;
                }
                Ok(None) | Err(rustix::io::Errno::INTR) => {}
                Err(_) => return Err(unavailable()),
            }
        }
        if !eof {
            let capacity = buffer
                .len()
                .min(output_limit.saturating_sub(output.len()).saturating_add(1));
            match stdout.read(&mut buffer[..capacity]) {
                Ok(0) => eof = true,
                Ok(size) => {
                    if size > output_limit.saturating_sub(output.len()) {
                        return Err(limit());
                    }
                    output.try_reserve_exact(size).map_err(|_| limit())?;
                    output.extend_from_slice(&buffer[..size]);
                    continue;
                }
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {}
                Err(_) => return Err(unavailable()),
            }
        }
        if exited && eof {
            child.reap()?;
            return Ok(output);
        }
        let mut descriptors = if eof {
            Vec::new()
        } else {
            vec![PollFd::new(&stdout, PollFlags::IN)]
        };
        let remaining = deadline.saturating_duration_since(Instant::now());
        let pause = remaining.min(Duration::from_millis(5));
        let timeout = Timespec {
            tv_sec: 0,
            tv_nsec: pause.subsec_nanos().into(),
        };
        match poll(&mut descriptors, Some(&timeout)) {
            Ok(_) | Err(rustix::io::Errno::INTR) => {}
            Err(_) => return Err(unavailable()),
        }
    }
}

struct OwnedGroup {
    child: Child,
    pid: Pid,
    terminated: bool,
    reaped: bool,
}
impl OwnedGroup {
    fn new(child: Child) -> Self {
        // A successfully spawned Linux child always has a positive pid_t.
        let pid = Pid::from_raw(child.id() as i32).expect("spawned child has a positive pid");
        Self {
            child,
            pid,
            terminated: false,
            reaped: false,
        }
    }
    fn terminate(&mut self) -> Result<(), ApplicationError> {
        if !self.terminated {
            match kill_process_group(self.pid, Signal::KILL) {
                Ok(()) | Err(rustix::io::Errno::SRCH) => self.terminated = true,
                Err(_) => return Err(unavailable()),
            }
        }
        Ok(())
    }
    fn reap(&mut self) -> Result<(), ApplicationError> {
        self.child.wait().map_err(|_| unavailable())?;
        self.reaped = true;
        Ok(())
    }
}
impl Drop for OwnedGroup {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.terminate();
            let _ = self.child.kill();
            let _ = self.reap();
        }
    }
}
