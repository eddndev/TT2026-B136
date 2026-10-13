//! Bounded transport to the fixed local CA worker, with no session credentials.
use application::{
    ca_access::OperatorIdentity,
    ca_operations::{CaCommand, CaExecutor},
    ApplicationError, PortFailureKind,
};
use domain::identity::UserId;
use rustix::{
    event::{poll, PollFd, PollFlags, Timespec},
    fs::{fcntl_getfl, fcntl_setfl, OFlags},
    process::{kill_process_group, waitid, Pid, Signal, WaitId, WaitIdOptions},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    ffi::OsString,
    io::{self, ErrorKind, Read, Write},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const INPUT_LIMIT: usize = 64 * 1024;
const OUTPUT_LIMIT: usize = 3 * 1024 * 1024;

fn unavailable() -> ApplicationError {
    ApplicationError::ClassifiedPort {
        kind: PortFailureKind::Unavailable,
        message: "CA worker is unavailable".into(),
    }
}

fn invalid() -> ApplicationError {
    ApplicationError::InvalidInput("CA operation input is invalid".into())
}

fn configuration() -> ApplicationError {
    ApplicationError::InvalidConfiguration("CA worker configuration is unavailable".into())
}

fn configured(
    path: PathBuf,
    directory: bool,
    executable: bool,
) -> Result<PathBuf, ApplicationError> {
    if !path.is_absolute() {
        return Err(configuration());
    }
    let path = path.canonicalize().map_err(|_| configuration())?;
    let info = path.metadata().map_err(|_| configuration())?;
    if (directory && !info.is_dir())
        || (!directory && !info.is_file())
        || (executable && info.permissions().mode() & 0o111 == 0)
    {
        return Err(configuration());
    }
    Ok(path)
}

pub struct NativeCaExecutor {
    python: PathBuf,
    arguments: Vec<OsString>,
    timeout: Duration,
}

impl NativeCaExecutor {
    pub fn new(
        python: PathBuf,
        worker: PathBuf,
        root: PathBuf,
        inspector: PathBuf,
        cli: PathBuf,
        scripts: PathBuf,
        expected_root: String,
    ) -> Result<Self, ApplicationError> {
        if expected_root.len() != 64
            || !expected_root
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(configuration());
        }
        let python = configured(python, false, true)?;
        let worker = configured(worker, false, false)?;
        let root = configured(root, true, false)?;
        let inspector = configured(inspector, false, true)?;
        let cli = configured(cli, false, true)?;
        let scripts = configured(scripts, true, false)?;
        let arguments = vec![
            "-I".into(),
            "-B".into(),
            "-S".into(),
            worker.into_os_string(),
            "--root".into(),
            root.into_os_string(),
            "--inspector".into(),
            inspector.into_os_string(),
            "--cli".into(),
            cli.into_os_string(),
            "--scripts".into(),
            scripts.into_os_string(),
            "--expected-root".into(),
            expected_root.into(),
        ];
        Ok(Self {
            python,
            arguments,
            timeout: Duration::from_secs(180),
        })
    }

    #[cfg(test)]
    #[allow(dead_code)] // Integration tests import this module; binary unit tests do not use it.
    pub(super) fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn run(&self, encoded: &[u8]) -> Result<Vec<u8>, ApplicationError> {
        let deadline = Instant::now() + self.timeout;
        let runtime = format!("/run/user/{}", rustix::process::geteuid().as_raw());
        let child = Command::new(&self.python)
            .args(&self.arguments)
            .env_clear()
            .env(
                "PATH",
                "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
            )
            .env("XDG_RUNTIME_DIR", &runtime)
            .env(
                "DBUS_SESSION_BUS_ADDRESS",
                format!("unix:path={runtime}/bus"),
            )
            .env("LC_ALL", "C")
            .env("TZ", "UTC")
            .current_dir("/")
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| unavailable())?;
        let mut child = OwnedGroup::new(child);
        let mut input = child.child.stdin.take();
        let mut output = child.child.stdout.take().ok_or_else(unavailable)?;
        let stdin = input.as_ref().ok_or_else(unavailable)?;
        fcntl_setfl(
            stdin,
            fcntl_getfl(stdin).map_err(|_| unavailable())? | OFlags::NONBLOCK,
        )
        .map_err(|_| unavailable())?;
        fcntl_setfl(
            &output,
            fcntl_getfl(&output).map_err(|_| unavailable())? | OFlags::NONBLOCK,
        )
        .map_err(|_| unavailable())?;
        let mut bytes = Vec::new();
        let mut buffer = [0; 8192];
        let (mut offset, mut eof, mut exited) = (0, false, false);
        loop {
            if Instant::now() >= deadline {
                return Err(unavailable());
            }
            if let Some(stdin) = input.as_mut() {
                match stdin.write(&encoded[offset..]) {
                    Ok(0) => return Err(unavailable()),
                    Ok(size) => {
                        offset += size;
                        if offset == encoded.len() {
                            input.take();
                        }
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            ErrorKind::WouldBlock | ErrorKind::Interrupted
                        ) => {}
                    Err(_) => return Err(unavailable()),
                }
            }
            if !exited {
                match waitid(
                    WaitId::Pid(child.pid),
                    WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
                ) {
                    Ok(Some(status)) => {
                        // Keep the leader unreaped until its group is stopped, so
                        // its process-group identity cannot be reused meanwhile.
                        child.terminate()?;
                        if status.exit_status() != Some(0) || offset != encoded.len() {
                            return Err(unavailable());
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
                    .min(OUTPUT_LIMIT.saturating_sub(bytes.len()) + 1);
                match output.read(&mut buffer[..capacity]) {
                    Ok(0) => eof = true,
                    Ok(size) => {
                        if size > OUTPUT_LIMIT.saturating_sub(bytes.len()) {
                            return Err(unavailable());
                        }
                        bytes.try_reserve_exact(size).map_err(|_| unavailable())?;
                        bytes.extend_from_slice(&buffer[..size]);
                        continue;
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            ErrorKind::WouldBlock | ErrorKind::Interrupted
                        ) => {}
                    Err(_) => return Err(unavailable()),
                }
            }
            if exited && eof {
                child.reap()?;
                return Ok(bytes);
            }
            let mut descriptors = Vec::with_capacity(2);
            if let Some(stdin) = &input {
                descriptors.push(PollFd::new(stdin, PollFlags::OUT));
            }
            if !eof {
                descriptors.push(PollFd::new(&output, PollFlags::IN));
            }
            let pause = deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(5));
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
}

#[derive(Serialize)]
struct Request<'a> {
    operator_id: UserId,
    command: &'a CaCommand,
}

struct BoundedInput(Vec<u8>);
impl Write for BoundedInput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > INPUT_LIMIT.saturating_sub(self.0.len()) {
            return Err(io::Error::other("CA input exceeds its bound"));
        }
        self.0
            .try_reserve_exact(bytes.len())
            .map_err(|_| io::Error::other("CA input unavailable"))?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Success {
    ok: bool,
    value: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Failure {
    ok: bool,
    code: FailureCode,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum FailureCode {
    Invalid,
    Busy,
    Unavailable,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Reply {
    Success(Success),
    Failure(Failure),
}

impl CaExecutor for NativeCaExecutor {
    fn execute(
        &self,
        actor: &OperatorIdentity,
        command: CaCommand,
    ) -> Result<Value, ApplicationError> {
        if actor.id.as_uuid().is_nil() {
            return Err(invalid());
        }
        let mut input = BoundedInput(Vec::new());
        serde_json::to_writer(
            &mut input,
            &Request {
                operator_id: actor.id,
                command: &command,
            },
        )
        .map_err(|_| invalid())?;
        let output = self.run(&input.0)?;
        match serde_json::from_slice::<Reply>(&output).map_err(|_| unavailable())? {
            Reply::Success(Success { ok: true, value }) => Ok(value),
            Reply::Failure(Failure {
                ok: false,
                code: FailureCode::Invalid,
            }) => Err(invalid()),
            Reply::Failure(Failure {
                ok: false,
                code: FailureCode::Busy,
            }) => Err(ApplicationError::ClassifiedPort {
                kind: PortFailureKind::Busy,
                message: "CA operation is busy".into(),
            }),
            _ => Err(unavailable()),
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
