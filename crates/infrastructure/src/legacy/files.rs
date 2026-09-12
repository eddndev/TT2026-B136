use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use application::ApplicationError;
use fs2::FileExt;

use super::{digest, invalid, ImportReport, LegacyImport};

pub(super) fn read_regular(path: &Path) -> Result<Vec<u8>, ApplicationError> {
    if !std::fs::symlink_metadata(path)
        .map_err(invalid)?
        .file_type()
        .is_file()
    {
        return Err(invalid(format!(
            "expected a regular file at {}",
            path.display()
        )));
    }
    std::fs::read(path).map_err(invalid)
}

pub(super) fn snapshot(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, ApplicationError> {
    let mut files = BTreeMap::new();
    let documents = root.join("documents");
    if documents.exists() {
        for entry in std::fs::read_dir(documents).map_err(invalid)? {
            let entry = entry.map_err(invalid)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".json") {
                files.insert(format!("documents/{name}"), read_regular(&entry.path())?);
            } else if !name.ends_with(".lock")
                && name != ".migrated"
                && name != ".migration-pending"
            {
                return Err(invalid(format!(
                    "unrecognized document storage entry {name}"
                )));
            }
        }
    }
    let audit = root.join("audit.jsonl");
    if audit.exists() {
        files.insert("audit.jsonl".into(), read_regular(&audit)?);
    }
    Ok(files)
}

pub(super) fn hashes(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|(path, bytes)| (path.clone(), digest(bytes)))
        .collect()
}

fn exclusive(path: &Path) -> Result<File, ApplicationError> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(invalid)?;
    file.lock_exclusive().map_err(invalid)?;
    Ok(file)
}

impl LegacyImport {
    pub(super) fn lock_source(&self) -> Result<(File, File), ApplicationError> {
        std::fs::create_dir_all(self.root.join("documents")).map_err(invalid)?;
        let documents = exclusive(&self.root.join("documents/.migration.lock"))?;
        let audit = exclusive(&self.root.join("audit.jsonl.lock"))?;
        if hashes(&snapshot(&self.root)?) != self.report.source_files
            || digest(&read_regular(&self.mapping)?) != self.mapping_hash
        {
            return Err(invalid("source changed after inspection; inspect again"));
        }
        Ok((documents, audit))
    }

    pub(super) fn prepare_source(&self) -> Result<(), ApplicationError> {
        for path in self.pending_paths() {
            write_marker(&path, &self.report)?;
        }
        Ok(())
    }

    fn pending_paths(&self) -> [std::path::PathBuf; 2] {
        [
            self.root.join("documents/.migration-pending"),
            self.root.join("audit.jsonl.migration-pending"),
        ]
    }

    pub(super) fn mark_source(&self) -> Result<(), ApplicationError> {
        for path in [
            self.root.join("documents/.migrated"),
            self.root.join("audit.jsonl.migrated"),
        ] {
            write_marker(&path, &self.report)?;
        }
        for path in self.pending_paths() {
            if path.exists() {
                std::fs::remove_file(&path).map_err(invalid)?;
                File::open(
                    path.parent()
                        .ok_or_else(|| invalid("missing marker parent"))?,
                )
                .and_then(|directory| directory.sync_all())
                .map_err(invalid)?;
            }
        }
        Ok(())
    }
}

fn write_marker(path: &Path, report: &ImportReport) -> Result<(), ApplicationError> {
    if path.exists() {
        let previous: ImportReport =
            serde_json::from_slice(&read_regular(path)?).map_err(invalid)?;
        if previous.fingerprint != report.fingerprint {
            return Err(invalid("different import marker exists"));
        }
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| invalid("missing marker parent"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(invalid)?;
    temporary
        .write_all(&serde_json::to_vec_pretty(report).map_err(invalid)?)
        .map_err(invalid)?;
    temporary.as_file().sync_all().map_err(invalid)?;
    temporary.persist_noclobber(path).map_err(invalid)?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(invalid)?;
    Ok(())
}

/// Refuses to start a database workflow over an unimported or changed local store.
pub fn require_completed_import(root: &Path, database_url: &str) -> Result<(), ApplicationError> {
    let files = snapshot(root)?;
    if files.is_empty() {
        return Ok(());
    }
    let marker = root.join("documents/.migrated");
    let report: ImportReport = serde_json::from_slice(&read_regular(&marker).map_err(|_| {
        invalid("local records exist; inspect and apply database import before serving")
    })?)
    .map_err(invalid)?;
    if hashes(&files) != report.source_files {
        return Err(invalid("legacy source changed after import"));
    }
    let audit_report: ImportReport =
        serde_json::from_slice(&read_regular(&root.join("audit.jsonl.migrated"))?)
            .map_err(invalid)?;
    if serde_json::to_value(&audit_report).map_err(invalid)?
        != serde_json::to_value(&report).map_err(invalid)?
    {
        return Err(invalid("import markers disagree"));
    }
    let mut client = crate::postgres::open(database_url)?;
    let mut transaction = client
        .build_transaction()
        .read_only(true)
        .isolation_level(postgres::IsolationLevel::RepeatableRead)
        .start()
        .map_err(invalid)?;
    let found = transaction.query_opt(
        "SELECT document_count, audit_count, audit_head FROM migration_receipts WHERE fingerprint=$1",
        &[&report.fingerprint],
    ).map_err(invalid)?;
    let Some(row) = found else {
        return Err(invalid(
            "database does not contain the source import receipt",
        ));
    };
    if row.get::<_, i64>(0) != report.documents as i64
        || row.get::<_, i64>(1) != report.audit_entries as i64
        || row.get::<_, String>(2) != report.audit_head
    {
        return Err(invalid("database import receipt disagrees"));
    }
    super::reconciliation::reconcile_source(&mut transaction, &report, &files)?;
    transaction.rollback().map_err(invalid)
}
