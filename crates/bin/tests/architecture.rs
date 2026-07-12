//! Architecture rule: the domain crate depends on no other workspace crate.
//!
//! The inner layer of the architecture must not reference the outer layers.
//! This test reads the domain manifest and fails if it declares a dependency
//! on any sibling crate.

use std::fs;
use std::path::PathBuf;

fn domain_manifest() -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/bin -> crates
    path.push("domain/Cargo.toml");
    fs::read_to_string(&path).expect("domain manifest is readable")
}

fn dependency_names(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
            continue;
        }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once(['=', '.']) {
            names.push(name.trim().to_string());
        }
    }
    names
}

#[test]
fn domain_has_no_workspace_dependencies() {
    let deps = dependency_names(&domain_manifest());
    for forbidden in ["application", "infrastructure", "web"] {
        assert!(
            !deps.iter().any(|dep| dep == forbidden),
            "domain must not depend on `{forbidden}`; found it in the manifest"
        );
    }
}
