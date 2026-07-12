//! Handlers for the content-integrity subcommands.

use std::fs::File;
use std::path::Path;

use anyhow::Context;
use application::HashDocument;
use infrastructure::RingSha256Hasher;

/// Hashes `file` with SHA-256 and prints the digest to standard output.
///
/// Human-readable output is the lower-case hex digest on its own line. With
/// `json` set, the output is one object carrying the algorithm name, the hex
/// digest, and the file path.
pub fn hash_file(file: &Path, json: bool) -> anyhow::Result<()> {
    let mut input =
        File::open(file).with_context(|| format!("cannot open file: {}", file.display()))?;
    let use_case = HashDocument::new(RingSha256Hasher::new());
    let digest = use_case.execute(&mut input)?;

    if json {
        let body = serde_json::json!({
            "algorithm": "sha-256",
            "digest": digest.to_hex(),
            "file": file.display().to_string(),
        });
        println!("{body}");
    } else {
        println!("{}", digest.to_hex());
    }
    Ok(())
}
