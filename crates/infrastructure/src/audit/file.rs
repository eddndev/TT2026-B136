//! JSON-lines file adapter for the audit log.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::PathBuf;

use domain::audit::{chain_digest, AuditEvent, AuditLog, ChainedEvent, GENESIS_PREVIOUS};
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::DomainError;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// [`AuditLog`] stored as one JSON object per line in a file.
///
/// Appending reads only the last line to link the new entry, then writes
/// the new line in append mode; nothing is recomputed or verified on
/// append, so a tampered file is only detected by a later full-chain
/// verification. That read-then-write step runs under an exclusive
/// advisory lock on a sibling `<log>.lock` file, so concurrent appenders
/// (threads or processes) serialize instead of linking to the same
/// previous entry and forking the chain. Loading holds a shared lock while
/// streaming the file so it never observes a partially appended JSON line.
/// A missing file is an empty log and is created by the first append.
pub struct FileAuditLog<H> {
    path: PathBuf,
    hasher: H,
}

/// Stored form of one entry. This is the storage encoding of the file
/// adapter only; the bytes covered by the hash chain are the canonical
/// encoding defined in the domain. The timestamp is the RFC 3339 UTC
/// string the canonical encoding hashes, so reading it back reproduces
/// the same chain input.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEntry {
    sequence: u64,
    timestamp: String,
    actor: String,
    action: String,
    resource: String,
    /// Chain value as lower-case hex.
    chain: String,
}

impl StoredEntry {
    fn from_chained(entry: &ChainedEvent) -> Result<Self, DomainError> {
        Ok(Self {
            sequence: entry.event.sequence,
            timestamp: entry.event.timestamp_rfc3339()?,
            actor: entry.event.actor.clone(),
            action: entry.event.action.clone(),
            resource: entry.event.resource.clone(),
            chain: entry.chain.to_hex(),
        })
    }

    fn into_chained(self, line_number: usize) -> Result<ChainedEvent, DomainError> {
        let timestamp = OffsetDateTime::parse(&self.timestamp, &Rfc3339).map_err(|err| {
            DomainError::AuditStorageFailure(format!(
                "line {line_number}: invalid rfc 3339 timestamp: {err}"
            ))
        })?;
        let chain = Sha256Digest::from_hex(&self.chain).map_err(|_| {
            DomainError::AuditStorageFailure(format!(
                "line {line_number}: chain value is not 64 hex characters"
            ))
        })?;
        Ok(ChainedEvent {
            event: AuditEvent::new(
                self.sequence,
                timestamp,
                self.actor,
                self.action,
                self.resource,
            ),
            chain,
        })
    }
}

impl<H: DocumentHasher> FileAuditLog<H> {
    /// Creates the adapter over the file at `path`; the file itself is only
    /// touched when entries are appended or loaded.
    pub fn new(path: impl Into<PathBuf>, hasher: H) -> Self {
        Self {
            path: path.into(),
            hasher,
        }
    }

    /// Reads the sequence number and chain value of the last stored entry,
    /// or `None` when the file does not exist or holds no entries.
    fn last_link(&self) -> Result<Option<(u64, Sha256Digest)>, DomainError> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(self.storage_failure(&err)),
        };
        let mut last: Option<(usize, String)> = None;
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|err| self.storage_failure(&err))?;
            if !line.trim().is_empty() {
                last = Some((index + 1, line));
            }
        }
        match last {
            None => Ok(None),
            Some((line_number, line)) => {
                let entry = parse_line(&line, line_number)?;
                Ok(Some((entry.event.sequence, entry.chain)))
            }
        }
    }

    /// Opens the sibling lock file shared by readers and appenders.
    fn open_lock_file(&self) -> Result<File, DomainError> {
        let mut lock_path = self.path.clone().into_os_string();
        lock_path.push(".lock");
        // The lock file's content is never used, only its lock state, so
        // an existing file is left as it is (no truncation).
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(PathBuf::from(lock_path))
            .map_err(|err| self.storage_failure(&err))
    }

    /// Holds exclusive access until the returned file handle is dropped.
    fn acquire_append_lock(&self) -> Result<File, DomainError> {
        let lock_file = self.open_lock_file()?;
        lock_file
            .lock_exclusive()
            .map_err(|err| self.storage_failure(&err))?;
        Ok(lock_file)
    }

    /// Shares a stable read with other readers and excludes partial appends.
    fn acquire_read_lock(&self) -> Result<File, DomainError> {
        let lock_file = self.open_lock_file()?;
        FileExt::lock_shared(&lock_file).map_err(|err| self.storage_failure(&err))?;
        Ok(lock_file)
    }

    fn storage_failure(&self, err: &dyn std::fmt::Display) -> DomainError {
        DomainError::AuditStorageFailure(format!("{}: {err}", self.path.display()))
    }
}

fn parse_line(line: &str, line_number: usize) -> Result<ChainedEvent, DomainError> {
    let stored: StoredEntry = serde_json::from_str(line).map_err(|err| {
        DomainError::AuditStorageFailure(format!("line {line_number}: not a stored entry: {err}"))
    })?;
    stored.into_chained(line_number)
}

impl<H: DocumentHasher> AuditLog for FileAuditLog<H> {
    fn append(
        &mut self,
        actor: &str,
        action: &str,
        resource: &str,
        timestamp: OffsetDateTime,
    ) -> Result<ChainedEvent, DomainError> {
        // The last link must be read under the same lock that guards the
        // write, so a racing appender's entry is observed and the new
        // entry links to it instead of forking the chain. The lock is
        // released when this guard is dropped at the end of the method.
        let _append_lock = self.acquire_append_lock()?;
        let mut marker = self.path.clone().into_os_string();
        marker.push(".migrated");
        let mut pending = self.path.clone().into_os_string();
        pending.push(".migration-pending");
        if PathBuf::from(marker).exists() || PathBuf::from(pending).exists() {
            return Err(self.storage_failure(&"audit log is fenced for migration and is read-only"));
        }
        let (sequence, previous) = match self.last_link()? {
            Some((last_sequence, last_chain)) => (last_sequence + 1, last_chain),
            None => (0, GENESIS_PREVIOUS),
        };
        let event = AuditEvent::new(sequence, timestamp, actor, action, resource);
        let chain = chain_digest(&self.hasher, &previous, &event)?;
        let entry = ChainedEvent { event, chain };

        let stored = StoredEntry::from_chained(&entry)?;
        let mut line = serde_json::to_string(&stored).map_err(|err| self.storage_failure(&err))?;
        line.push('\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|err| self.storage_failure(&err))?;
        file.write_all(line.as_bytes())
            .map_err(|err| self.storage_failure(&err))?;
        Ok(entry)
    }

    fn load_all(&self) -> Result<Vec<ChainedEvent>, DomainError> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(self.storage_failure(&err)),
        };
        let _read_lock = self.acquire_read_lock()?;
        let mut entries = Vec::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|err| self.storage_failure(&err))?;
            if line.trim().is_empty() {
                continue;
            }
            entries.push(parse_line(&line, index + 1)?);
        }
        Ok(entries)
    }
}

/// Parses a stopped legacy writer's snapshot without creating lock files.
pub fn decode_audit_snapshot(bytes: &[u8]) -> Result<Vec<ChainedEvent>, DomainError> {
    BufReader::new(bytes)
        .lines()
        .enumerate()
        .filter_map(|(index, line)| match line {
            Ok(line) if line.trim().is_empty() => None,
            Ok(line) => Some(parse_line(&line, index + 1)),
            Err(error) => Some(Err(DomainError::AuditStorageFailure(error.to_string()))),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::hashing::RingSha256Hasher;

    fn timestamp(offset_seconds: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_735_689_600 + offset_seconds).unwrap()
    }

    fn log_at(path: &Path) -> FileAuditLog<RingSha256Hasher> {
        FileAuditLog::new(path, RingSha256Hasher::new())
    }

    #[test]
    fn a_missing_file_is_an_empty_log() {
        let dir = tempfile::tempdir().unwrap();
        let log = log_at(&dir.path().join("missing.jsonl"));
        assert_eq!(log.load_all().unwrap(), vec![]);
    }

    #[test]
    fn the_first_append_creates_the_file_with_one_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut log = log_at(&path);

        let entry = log.append("ana", "open", "case-1", timestamp(0)).unwrap();

        assert_eq!(entry.event.sequence, 0);
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), 1);
    }

    #[test]
    fn stored_lines_are_json_objects_with_an_rfc3339_utc_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        log_at(&path)
            .append("ana", "open", "case-1", timestamp(0))
            .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(value["sequence"], 0);
        assert_eq!(value["timestamp"], "2025-01-01T00:00:00Z");
        assert_eq!(value["actor"], "ana");
        assert_eq!(value["action"], "open");
        assert_eq!(value["resource"], "case-1");
        assert_eq!(value["chain"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn load_all_round_trips_what_append_wrote() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut log = log_at(&path);
        let first = log.append("ana", "open", "case-1", timestamp(0)).unwrap();
        let second = log.append("bob", "close", "case-2", timestamp(1)).unwrap();

        assert_eq!(log.load_all().unwrap(), vec![first, second]);
    }

    #[test]
    fn a_line_that_is_not_json_surfaces_as_a_storage_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        std::fs::write(&path, "not json\n").unwrap();

        let err = log_at(&path).load_all().unwrap_err();
        match err {
            DomainError::AuditStorageFailure(message) => {
                assert!(message.contains("line 1"), "message: {message}");
            }
            other => panic!("expected a storage failure, got: {other}"),
        }
    }

    #[test]
    fn a_malformed_chain_value_surfaces_as_a_storage_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let mut log = log_at(&path);
        log.append("ana", "open", "case-1", timestamp(0)).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, content.replace("\"chain\":\"", "\"chain\":\"zz")).unwrap();

        let err = log_at(&path).load_all().unwrap_err();
        assert!(matches!(err, DomainError::AuditStorageFailure(_)));
    }
}
