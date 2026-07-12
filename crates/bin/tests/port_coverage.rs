//! Architecture rule: every outbound port declared in the domain crate has
//! a concrete adapter in the infrastructure crate and a mockall-based mock
//! in the workspace's tests.
//!
//! A port merged without an adapter cannot be wired in the composition
//! root; a port merged without a mock cannot be exercised by use-case
//! tests. This test fails the build when either half is missing, so the
//! gap is caught in the change that introduces the port instead of when
//! the wiring is attempted later.
//!
//! The scan is line based, not a Rust parser. It collects `pub trait`
//! declarations from the domain sources and looks for `impl <Trait> for`
//! headers and `mock!` macro blocks in the rest of the workspace. That is
//! enough to catch a missing adapter or mock; it would be fooled by traits
//! generated through macros, by impl headers split across lines, or by
//! `mock!` appearing inside string literals, none of which this workspace
//! uses in production or test code.

use std::fs;
use std::path::{Path, PathBuf};

/// Public domain traits that are not outbound ports and therefore need
/// neither an adapter nor a mock. The list is intentionally explicit and
/// currently empty: every `pub trait` in the domain crate today is a port.
/// Any name added here must carry a comment explaining why the trait is
/// exempt.
const NON_PORT_TRAITS: &[&str] = &[];

/// Ports whose disappearance from the scan signals a broken scanner, not a
/// removed port. If the directory walk or the line scan stops working, the
/// coverage checks below would pass vacuously; these names keep the test
/// honest. Update the list if one of these ports is ever renamed.
const SENTINEL_PORTS: &[&str] = &["AuditLog", "DocumentHasher", "KeyManager"];

fn crates_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/bin -> crates
    path
}

/// Collects every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|err| panic!("cannot list {}: {err}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Reads the `.rs` files under each existing directory in `dirs`.
fn load_sources(dirs: &[PathBuf]) -> Vec<String> {
    let mut paths = Vec::new();
    for dir in dirs {
        if dir.is_dir() {
            rust_files(dir, &mut paths);
        }
    }
    paths
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()))
        })
        .collect()
}

/// The `src` and `tests` directories of every workspace crate.
fn crate_source_and_test_dirs() -> Vec<PathBuf> {
    let root = crates_root();
    let entries =
        fs::read_dir(&root).unwrap_or_else(|err| panic!("cannot list {}: {err}", root.display()));
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path.join("src"));
            dirs.push(path.join("tests"));
        }
    }
    dirs
}

/// Extracts the trait name from a `pub trait Name ...` line, if any.
fn declared_trait_name(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("pub trait ")?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Every outbound port name declared in the domain crate.
fn domain_port_names() -> Vec<String> {
    let sources = load_sources(&[crates_root().join("domain").join("src")]);
    let mut names = Vec::new();
    for text in &sources {
        for line in text.lines() {
            if let Some(name) = declared_trait_name(line) {
                if !NON_PORT_TRAITS.contains(&name.as_str()) {
                    names.push(name);
                }
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// True when some source has an `impl <name> for ...` header. Matches
/// generic headers too, such as `impl<H: Hasher> Port for Adapter<H>`.
fn has_impl_header(name: &str, sources: &[String]) -> bool {
    let needle = format!("{name} for ");
    sources.iter().any(|text| {
        text.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("impl") && trimmed.contains(&needle)
        })
    })
}

/// Returns each `mock!` macro invocation in `text`, from the macro name
/// through its matching closing brace, found by brace counting.
fn mock_blocks(text: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut search_from = 0;
    while let Some(found) = text[search_from..].find("mock!") {
        let start = search_from + found;
        let Some(open_offset) = text[start..].find('{') else {
            break;
        };
        let open = start + open_offset;
        let mut depth = 0usize;
        let mut end = None;
        for (i, c) in text[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(open + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            break;
        };
        blocks.push(&text[start..end]);
        search_from = end;
    }
    blocks
}

/// True when a mock for the port exists: either a `mock!` block declares
/// `impl <name> for ...`, or some source references the generated
/// `Mock<name>` type directly.
fn has_mock(name: &str, sources: &[String]) -> bool {
    let generated_type = format!("Mock{name}");
    let mocked_impl = format!("impl {name} for ");
    sources.iter().any(|text| {
        text.contains(&generated_type)
            || mock_blocks(text)
                .iter()
                .any(|block| block.contains(&mocked_impl))
    })
}

#[test]
fn every_domain_port_has_an_infrastructure_adapter_and_a_mock() {
    let ports = domain_port_names();
    for sentinel in SENTINEL_PORTS {
        assert!(
            ports.iter().any(|port| port == sentinel),
            "the scan did not find the known port `{sentinel}`; \
             the scanner is broken or the port was renamed"
        );
    }

    let adapter_sources = load_sources(&[crates_root().join("infrastructure").join("src")]);
    let workspace_sources = load_sources(&crate_source_and_test_dirs());

    let mut failures = Vec::new();
    for port in &ports {
        if !has_impl_header(port, &adapter_sources) {
            failures.push(format!(
                "port `{port}` has no adapter: no `impl {port} for` \
                 under crates/infrastructure/src"
            ));
        }
        if !has_mock(port, &workspace_sources) {
            failures.push(format!(
                "port `{port}` has no mock: no `mock!` block implementing \
                 it and no `Mock{port}` reference under crates/*/src or \
                 crates/*/tests"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "domain ports without full coverage:\n{}",
        failures.join("\n")
    );
}
