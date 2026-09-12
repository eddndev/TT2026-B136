//! Administrative database commands and offline import options.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum DatabaseAction {
    /// Apply schema with DATABASE_URL and grant an existing restricted runtime role.
    Migrate {
        #[arg(long)]
        runtime_role: String,
    },
    /// Validate a legacy snapshot and destination; write only with --apply.
    Import {
        #[arg(long, default_value = "runtime-data")]
        data_dir: PathBuf,
        /// Explicit JSON document-to-case assignments.
        #[arg(long)]
        mapping: PathBuf,
        /// Commit the import and freeze the source files after validation.
        #[arg(long)]
        apply: bool,
    },
}
