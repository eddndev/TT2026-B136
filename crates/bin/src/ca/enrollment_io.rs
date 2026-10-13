//! Owner-only delivery and bounded private input for initial CA enrollment.
use std::{
    fs::{File, Metadata, OpenOptions, Permissions},
    io::{self, IsTerminal, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use rustix::termios::{self, InputModes, LocalModes, OptionalActions, SpecialCodeIndex, Termios};
use zeroize::Zeroizing;

pub struct PrivateInput {
    stdin: io::Stdin,
    original: Option<Termios>,
}

impl PrivateInput {
    pub fn open() -> anyhow::Result<Self> {
        let stdin = io::stdin();
        let original = if stdin.is_terminal() {
            let original = termios::tcgetattr(&stdin)?;
            let mut private = original.clone();
            // Read cancellation bytes ourselves so the guard restores echo.
            private.local_modes.remove(
                LocalModes::ECHO
                    | LocalModes::ECHONL
                    | LocalModes::ICANON
                    | LocalModes::ISIG
                    | LocalModes::IEXTEN,
            );
            private.input_modes.remove(InputModes::IXON);
            private.special_codes[SpecialCodeIndex::VMIN] = 1;
            private.special_codes[SpecialCodeIndex::VTIME] = 0;
            termios::tcsetattr(&stdin, OptionalActions::Now, &private)?;
            Some(original)
        } else {
            None
        };
        Ok(Self { stdin, original })
    }

    pub fn notice(&self, message: &str) -> anyhow::Result<()> {
        if self.original.is_some() {
            writeln!(io::stderr().lock(), "{message}")?;
        }
        Ok(())
    }

    pub fn line(&self, prompt: &str, limit: usize) -> anyhow::Result<Zeroizing<String>> {
        if self.original.is_some() {
            let mut stderr = io::stderr().lock();
            write!(stderr, "{prompt}: ")?;
            stderr.flush()?;
        }
        let mut bytes = Zeroizing::new(Vec::<u8>::with_capacity(limit + 1));
        let mut byte = Zeroizing::new([0_u8; 1]);
        loop {
            // Avoid the standard input buffer retaining secret lines after use.
            if rustix::io::read(&self.stdin, &mut *byte)? == 0 {
                bail!("private enrollment input ended before confirmation");
            }
            match byte[0] {
                b'\n' => break,
                b'\r' if self.original.is_some() => break,
                3 | 4 | 26 => bail!("private enrollment input was cancelled"),
                8 | 127 if self.original.is_some() => {
                    if let Some(mut removed) = bytes.pop() {
                        use zeroize::Zeroize;
                        removed.zeroize();
                    }
                }
                value if value < 32 && value != b'\r' => {
                    bail!("private enrollment input contains unsupported control bytes");
                }
                value => bytes.push(value),
            }
            if bytes.len() > limit + 1 {
                bail!("private enrollment input exceeds its bound");
            }
        }
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
        if bytes.len() > limit || bytes.iter().any(|value| *value < 32 || *value == 127) {
            bail!("private enrollment input is invalid");
        }
        if self.original.is_some() {
            writeln!(io::stderr().lock())?;
        }
        let value = std::str::from_utf8(&bytes)
            .map_err(|_| anyhow::anyhow!("private enrollment input is not UTF-8"))?;
        Ok(Zeroizing::new(value.to_owned()))
    }
}

impl Drop for PrivateInput {
    fn drop(&mut self) {
        if let Some(original) = &self.original {
            let _ = termios::tcsetattr(&self.stdin, OptionalActions::Now, original);
        }
    }
}

pub struct PrivateDelivery {
    file: File,
    path: PathBuf,
    bytes: Zeroizing<Vec<u8>>,
}

impl PrivateDelivery {
    pub fn create(path: &Path) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(path)
            .context("enrollment delivery requires a new private file")?;
        file.set_permissions(Permissions::from_mode(0o600))?;
        validate_private(&file.metadata()?)?;
        Ok(Self {
            file,
            path: path.to_owned(),
            bytes: Zeroizing::new(Vec::new()),
        })
    }

    pub fn persist(&mut self, bytes: Zeroizing<Vec<u8>>) -> anyhow::Result<()> {
        self.bytes = bytes;
        self.file.write_all(&self.bytes)?;
        self.file.sync_all()?;
        let parent = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        File::open(parent)?.sync_all()?;
        self.verify()
    }

    /// Recheck the pathname, permissions and exact payload immediately before commit.
    pub fn verify(&self) -> anyhow::Result<()> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(
                (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
            )
            .open(&self.path)
            .context("private enrollment delivery is unavailable")?;
        let expected = self.file.metadata()?;
        let actual = file.metadata()?;
        validate_private(&expected)?;
        validate_private(&actual)?;
        if expected.dev() != actual.dev()
            || expected.ino() != actual.ino()
            || actual.len() != self.bytes.len() as u64
            || self.bytes.is_empty()
        {
            bail!("private enrollment delivery was replaced");
        }
        let mut persisted = Zeroizing::new(Vec::new());
        file.take(self.bytes.len() as u64 + 1)
            .read_to_end(&mut persisted)?;
        if *persisted != *self.bytes {
            bail!("private enrollment delivery changed");
        }
        Ok(())
    }
}

fn validate_private(info: &Metadata) -> anyhow::Result<()> {
    if !info.is_file()
        || info.mode() & 0o7777 != 0o600
        || info.nlink() != 1
        || info.uid() != rustix::process::geteuid().as_raw()
    {
        bail!("enrollment delivery must be an owned private regular file");
    }
    Ok(())
}
