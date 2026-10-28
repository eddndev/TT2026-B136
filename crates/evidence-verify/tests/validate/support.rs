//! Shared helpers: fixture access, a stored ZIP writer with the layout of
//! `crates/infrastructure/src/archive.rs`, package builders and assertions.

use evidence_verify::json::{self, Limits, Value};
use evidence_verify::policy::{PinnedRoot, Policy, Purposes};
use evidence_verify::report::{Component, ComponentKind, Report};
use evidence_verify::Outcome;

/// Bytes of one file under `tests/fixtures`.
macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/",
            $name
        ))
        .as_slice()
    };
}
pub(crate) use fixture;

/// Name the document takes inside every test package.
pub const DOCUMENT: &str = "acta.pdf";

/// Facts recorded by the fixture generator that the tests depend on.
pub struct Facts {
    pub gen_time: i64,
    pub accuracy: i64,
    pub crl_this_update: i64,
    pub crl_next_update: i64,
    pub certificate_not_after: i64,
    pub root_sha256: [u8; 32],
    pub foreign_root_sha256: [u8; 32],
    pub document_sha256: String,
}

pub fn facts() -> Facts {
    // The crate parser admits only the canonical subset, without literals,
    // so the generator's literals become strings before parsing.
    let text = std::str::from_utf8(fixture!("fixtures.json"))
        .expect("fixtures.json is UTF-8")
        .replace(": null", ": \"null\"")
        .replace(": true", ": \"true\"")
        .replace(": false", ": \"false\"");
    let value =
        json::parse(text.as_bytes(), Limits { max_values: 10_000 }).expect("fixtures.json parses");
    let int = |path: &[&str]| -> i64 {
        let mut at: &Value = &value;
        for key in path {
            at = at.get(key).expect("fixture fact");
        }
        i64::try_from(at.as_integer().expect("integer fact")).expect("fits")
    };
    let digest = |file: &str| {
        let text = value
            .get("certificates")
            .and_then(|all| all.get(file))
            .and_then(|cert| cert.get("sha256"))
            .and_then(Value::as_str)
            .expect("certificate digest");
        unhex(text)
    };
    Facts {
        gen_time: int(&["tokens", "token-ess-v1.tsr", "gen_time"]),
        accuracy: int(&["tokens", "token-ess-v1.tsr", "accuracy_seconds"]),
        crl_this_update: int(&["crls", "root.crl.der", "this_update"]),
        crl_next_update: int(&["crls", "root.crl.der", "next_update"]),
        certificate_not_after: int(&["certificates", "holder.der", "not_after"]),
        root_sha256: digest("root.der"),
        foreign_root_sha256: digest("foreign-root.der"),
        document_sha256: value
            .get("document")
            .and_then(|document| document.get("sha256"))
            .and_then(Value::as_str)
            .expect("document digest")
            .to_owned(),
    }
}

pub fn unhex(text: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).expect("hex");
    }
    out
}

/// A policy that pins the fixture root for every purpose, evaluated one
/// hour after the fixture tokens were issued.
pub fn policy() -> Policy {
    let facts = facts();
    Policy::new(
        vec![pin(facts.root_sha256, Purposes::ALL, "fixture root")],
        facts.gen_time + 3600,
    )
}

pub fn pin(sha256: [u8; 32], purposes: Purposes, label: &str) -> PinnedRoot {
    PinnedRoot {
        sha256,
        purposes,
        label: label.to_owned(),
    }
}

/// Named entries in archive order, written as a stored ZIP on demand.
#[derive(Clone)]
pub struct Package {
    pub entries: Vec<(String, Vec<u8>)>,
}

impl Package {
    pub fn new(entries: &[(&str, &[u8])]) -> Self {
        Self {
            entries: entries
                .iter()
                .map(|(name, data)| ((*name).to_owned(), data.to_vec()))
                .collect(),
        }
    }

    /// A server seal export over `document.bin` with the given token, in
    /// the order the server writes it.
    pub fn server_seal(token: &[u8]) -> Self {
        Self::new(&[
            (DOCUMENT, fixture!("document.bin")),
            ("acta.pdf.sig", fixture!("server-signer.sig")),
            ("acta.pdf.tsr", token),
            ("certificado.pem", fixture!("server-signer.pem")),
            ("ca.pem", fixture!("root.pem")),
            ("crl.pem", fixture!("root.crl.pem")),
            ("tsa-chain.pem", fixture!("root.pem")),
            ("INSTRUCCIONES.md", b"# Instrucciones\n"),
        ])
    }

    pub fn get(&self, name: &str) -> &[u8] {
        &self
            .entries
            .iter()
            .find(|(entry, _)| entry == name)
            .expect("entry present")
            .1
    }

    /// Replace the content of `name`, or append it when absent.
    pub fn set(&mut self, name: &str, data: &[u8]) -> &mut Self {
        match self.entries.iter_mut().find(|(entry, _)| entry == name) {
            Some(entry) => entry.1 = data.to_vec(),
            None => self.entries.push((name.to_owned(), data.to_vec())),
        }
        self
    }

    pub fn remove(&mut self, name: &str) -> &mut Self {
        self.entries.retain(|(entry, _)| entry != name);
        self
    }

    pub fn zip(&self) -> Vec<u8> {
        let borrowed: Vec<(&str, &[u8])> = self
            .entries
            .iter()
            .map(|(name, data)| (name.as_str(), data.as_slice()))
            .collect();
        zip(&borrowed)
    }
}

/// Writes `entries` as the server's stored ZIP writer does: method 0,
/// flags 0, version needed 10, made by 20, DOS date 1980-01-01 at time 0,
/// no extra fields or comments, then the central directory and end record.
pub fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        let head = |out: &mut Vec<u8>| {
            u16s(out, &[0, 0, 0, 0x0021]);
            u32s(out, &[crc, size, size]);
            u16s(out, &[name.len() as u16, 0]);
        };
        u32s(&mut out, &[0x0403_4b50]);
        u16s(&mut out, &[10]);
        head(&mut out);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        u32s(&mut directory, &[0x0201_4b50]);
        u16s(&mut directory, &[20, 10]);
        head(&mut directory);
        u16s(&mut directory, &[0, 0, 0]);
        u32s(&mut directory, &[0, offset]);
        directory.extend_from_slice(name.as_bytes());
    }
    let directory_offset = out.len() as u32;
    let count = entries.len() as u16;
    out.extend_from_slice(&directory);
    u32s(&mut out, &[0x0605_4b50]);
    u16s(&mut out, &[0, 0, count, count]);
    u32s(&mut out, &[directory.len() as u32, directory_offset]);
    u16s(&mut out, &[0]);
    out
}

fn u16s(out: &mut Vec<u8>, values: &[u16]) {
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

fn u32s(out: &mut Vec<u8>, values: &[u32]) {
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

pub fn component(report: &Report, kind: ComponentKind) -> &Component {
    report
        .component(kind)
        .unwrap_or_else(|| panic!("no {kind:?} component in {report:#?}"))
}

/// Assert the outcome of one component and that it carries `code`.
pub fn assert_component(report: &Report, kind: ComponentKind, outcome: Outcome, code: &str) {
    let found = component(report, kind);
    assert_eq!(found.outcome, outcome, "{kind:?}: {report:#?}");
    assert!(
        found.findings.iter().any(|finding| finding.code == code),
        "{kind:?} lacks {code}: {report:#?}"
    );
}

/// Assert every mandatory component of `report` is checked.
pub fn assert_complete(report: &Report) {
    assert_eq!(report.overall, Outcome::Checked, "{report:#?}");
    for component in &report.components {
        assert!(
            matches!(component.outcome, Outcome::Checked | Outcome::NotApplicable),
            "{report:#?}"
        );
    }
}

/// Flip the last byte of `data`.
pub fn altered(data: &[u8]) -> Vec<u8> {
    let mut copy = data.to_vec();
    let last = copy.len() - 1;
    copy[last] ^= 0x01;
    copy
}
