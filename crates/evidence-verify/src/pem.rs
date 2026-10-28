//! Strict decoding of PEM text as the server writes it.
//!
//! The server writes each object as `-----BEGIN {label}-----`, the standard
//! base64 encoding with padding in lines of 64 characters, and
//! `-----END {label}-----`, every line ending in LF. This decoder admits that
//! form and the small set of variations a faithful copy can introduce: CRLF
//! line ends, shorter base64 lines, a missing final newline and whitespace-only
//! lines between blocks. It rejects anything that could hide or alter content:
//! another label, text outside blocks, encapsulated headers, non-canonical
//! base64, empty blocks and more blocks than the caller admits. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

mod base64;
#[cfg(test)]
mod tests;

use core::fmt;

/// Why a PEM input was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PemError {
    /// A byte is not printable ASCII, tab, or part of an LF or CRLF line end.
    NotText,
    /// A line outside every block holds more than spaces and tabs.
    TextOutsideBlock,
    /// A boundary names a label other than the expected one.
    WrongLabel,
    /// A line starts like a boundary but is not a well-formed one.
    MalformedBoundary,
    /// A block carries encapsulated headers such as `Proc-Type`.
    Headers,
    /// A base64 line is longer than 64 characters.
    LineTooLong,
    /// The block content is not canonical standard base64 with padding.
    InvalidBase64,
    /// A block has no content.
    EmptyBlock,
    /// The input ends inside a block.
    Unterminated,
    /// The input holds no block.
    NoBlock,
    /// The input holds more blocks than the caller admits.
    TooManyBlocks,
}

impl PemError {
    /// Stable machine-readable code for this error.
    pub fn code(self) -> &'static str {
        match self {
            Self::NotText => "pem.not_text",
            Self::TextOutsideBlock => "pem.text_outside_block",
            Self::WrongLabel => "pem.wrong_label",
            Self::MalformedBoundary => "pem.malformed_boundary",
            Self::Headers => "pem.headers",
            Self::LineTooLong => "pem.line_too_long",
            Self::InvalidBase64 => "pem.invalid_base64",
            Self::EmptyBlock => "pem.empty_block",
            Self::Unterminated => "pem.unterminated",
            Self::NoBlock => "pem.no_block",
            Self::TooManyBlocks => "pem.too_many_blocks",
        }
    }
}

impl fmt::Display for PemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for PemError {}

/// The longest base64 line admitted inside a block.
pub const MAX_LINE: usize = 64;

/// Decode every block of `input` labelled `label`, in order.
///
/// The input must hold between one and `max_blocks` blocks, all with the
/// label `label` compared byte for byte.
pub fn decode_all(input: &[u8], label: &str, max_blocks: usize) -> Result<Vec<Vec<u8>>, PemError> {
    let text = admitted_text(input)?;
    let mut blocks = Vec::new();
    let mut open: Option<String> = None;
    for line in text.split_terminator('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        match open.as_mut() {
            None => {
                if line.bytes().all(|byte| byte == b' ' || byte == b'\t') {
                    continue;
                }
                match boundary(line)? {
                    Some(Boundary::Begin(found)) if found != label => {
                        return Err(PemError::WrongLabel)
                    }
                    Some(Boundary::Begin(_)) if blocks.len() >= max_blocks => {
                        return Err(PemError::TooManyBlocks)
                    }
                    Some(Boundary::Begin(_)) => open = Some(String::new()),
                    Some(Boundary::End(_)) => return Err(PemError::MalformedBoundary),
                    None => return Err(PemError::TextOutsideBlock),
                }
            }
            Some(content) => match boundary(line)? {
                Some(Boundary::End(found)) if found != label => return Err(PemError::WrongLabel),
                Some(Boundary::End(_)) => {
                    blocks.push(block_bytes(content)?);
                    open = None;
                }
                Some(Boundary::Begin(_)) => return Err(PemError::MalformedBoundary),
                None => content.push_str(content_line(line)?),
            },
        }
    }
    if open.is_some() {
        return Err(PemError::Unterminated);
    }
    if blocks.is_empty() {
        return Err(PemError::NoBlock);
    }
    Ok(blocks)
}

/// Decode the single block of `input` labelled `label`.
pub fn decode_one(input: &[u8], label: &str) -> Result<Vec<u8>, PemError> {
    let mut blocks = decode_all(input, label, 1)?;
    blocks.pop().ok_or(PemError::NoBlock)
}

enum Boundary<'a> {
    Begin(&'a str),
    End(&'a str),
}

/// Admit printable ASCII, tab, LF and CR only as the first half of CRLF.
fn admitted_text(input: &[u8]) -> Result<&str, PemError> {
    for (index, &byte) in input.iter().enumerate() {
        let admitted = match byte {
            b'\t' | b'\n' | b' '..=b'~' => true,
            b'\r' => input.get(index + 1) == Some(&b'\n'),
            _ => false,
        };
        if !admitted {
            return Err(PemError::NotText);
        }
    }
    core::str::from_utf8(input).map_err(|_| PemError::NotText)
}

/// Classify a line that starts with five dashes as a boundary.
fn boundary(line: &str) -> Result<Option<Boundary<'_>>, PemError> {
    if !line.starts_with("-----") {
        return Ok(None);
    }
    let inner = line
        .strip_prefix("-----")
        .and_then(|rest| rest.strip_suffix("-----"))
        .ok_or(PemError::MalformedBoundary)?;
    if let Some(label) = inner.strip_prefix("BEGIN ") {
        Ok(Some(Boundary::Begin(label)))
    } else if let Some(label) = inner.strip_prefix("END ") {
        Ok(Some(Boundary::End(label)))
    } else {
        Err(PemError::MalformedBoundary)
    }
}

/// Check one line of block content and return it for concatenation.
fn content_line(line: &str) -> Result<&str, PemError> {
    if line.contains(':') {
        return Err(PemError::Headers);
    }
    let base64_symbol =
        |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=');
    if line.is_empty() || !line.bytes().all(base64_symbol) {
        return Err(PemError::InvalidBase64);
    }
    if line.len() > MAX_LINE {
        return Err(PemError::LineTooLong);
    }
    Ok(line)
}

fn block_bytes(content: &str) -> Result<Vec<u8>, PemError> {
    if content.is_empty() {
        return Err(PemError::EmptyBlock);
    }
    base64::decode(content.as_bytes()).ok_or(PemError::InvalidBase64)
}
