//! Admission tests: a round trip through a byte-exact copy of the server's
//! writer, and one mutation per rejection rule.

mod headers;
mod limits;
mod round_trip;
mod structure;
mod writer;

use super::admit;
use crate::report::{Finding, Outcome};

/// Admission must refuse `bytes` as not supported with `code`.
fn assert_not_supported(bytes: &[u8], code: &str) {
    let finding = refusal(bytes);
    assert_eq!(finding.outcome, Outcome::NotSupported, "{finding:?}");
    assert_eq!(finding.code, code, "{finding:?}");
}

fn refusal(bytes: &[u8]) -> Finding {
    match admit(bytes) {
        Ok(archive) => panic!("admitted {:?}", archive.names()),
        Err(finding) => finding,
    }
}

/// A small package with the shape of a server seal export.
fn sample() -> writer::Built {
    writer::write(&[
        ("acta.pdf", b"%PDF-1.7 body"),
        ("acta.pdf.sig", b"signature"),
        ("acta.pdf.tsr", b"token"),
        ("certificado.pem", b"-----BEGIN CERTIFICATE-----"),
        ("ca.pem", b"ca"),
        ("crl.pem", b""),
        ("INSTRUCCIONES.md", b"# Instrucciones"),
    ])
}
