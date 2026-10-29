//! Strict bounded JSON for evidence entries and its RFC 8785 serialization.
//!
//! The admitted subset is objects, arrays, strings and integers written
//! without fraction or exponent. Literals (`true`, `false`, `null`), floats,
//! exponents, leading zeros, `-0`, duplicate object keys, trailing data,
//! invalid UTF-8, unpaired surrogates and raw control characters in strings
//! are rejected. Nesting and the number of values are bounded. Object members
//! keep the order in which they were parsed. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

mod canonical;
mod parser;

#[cfg(test)]
mod canonical_tests;
#[cfg(test)]
mod parser_tests;

pub use canonical::{canonical, parse_canonical};
pub use parser::parse;

/// Deepest admitted nesting of arrays and objects; the outermost container
/// is at depth 1.
pub const MAX_DEPTH: usize = 32;

/// Largest integer admitted by the canonical form, 2^53 - 1.
pub const MAX_CANONICAL_INTEGER: i128 = (1 << 53) - 1;

/// One parsed JSON value of the admitted subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Members in parsed order; keys are unique.
    Object(Vec<(String, Value)>),
    Array(Vec<Value>),
    String(String),
    /// Always within `i64::MIN..=u64::MAX` when produced by [`parse`].
    Integer(i128),
}

impl Value {
    /// The value of member `key` when this is an object that has it.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Object(members) => Some(members),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_integer(&self) -> Option<i128> {
        match self {
            Value::Integer(number) => Some(*number),
            _ => None,
        }
    }
}

/// Caller-supplied bounds for one parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Most values admitted in the whole document, counting the root, every
    /// array item and every object member value. Keys are not counted.
    pub max_values: usize,
}

/// Why a document was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidUtf8,
    UnexpectedEnd,
    UnexpectedByte,
    TrailingData,
    /// `true`, `false` or `null`.
    Literal,
    /// A number with a fraction or an exponent.
    NotInteger,
    LeadingZero,
    NegativeZero,
    /// An integer outside `i64::MIN..=u64::MAX` when parsing, or outside
    /// `0..=MAX_CANONICAL_INTEGER` when serializing canonically.
    IntegerOutOfRange,
    InvalidEscape,
    UnpairedSurrogate,
    ControlCharacter,
    DuplicateKey,
    TooDeep,
    TooManyValues,
    /// Admitted bytes that are not the RFC 8785 serialization of their own
    /// value.
    NotCanonical,
}

/// A rejection with the byte offset where it was detected; canonical
/// serialization reports offset 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonError {
    pub kind: ErrorKind,
    pub offset: usize,
}

impl JsonError {
    pub(crate) fn new(kind: ErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    /// Stable ASCII code for reports.
    pub fn code(&self) -> &'static str {
        match self.kind {
            ErrorKind::InvalidUtf8 => "json.invalid_utf8",
            ErrorKind::UnexpectedEnd => "json.unexpected_end",
            ErrorKind::UnexpectedByte => "json.unexpected_byte",
            ErrorKind::TrailingData => "json.trailing_data",
            ErrorKind::Literal => "json.literal",
            ErrorKind::NotInteger => "json.not_integer",
            ErrorKind::LeadingZero => "json.leading_zero",
            ErrorKind::NegativeZero => "json.negative_zero",
            ErrorKind::IntegerOutOfRange => "json.integer_out_of_range",
            ErrorKind::InvalidEscape => "json.invalid_escape",
            ErrorKind::UnpairedSurrogate => "json.unpaired_surrogate",
            ErrorKind::ControlCharacter => "json.control_character",
            ErrorKind::DuplicateKey => "json.duplicate_key",
            ErrorKind::TooDeep => "json.too_deep",
            ErrorKind::TooManyValues => "json.too_many_values",
            ErrorKind::NotCanonical => "json.not_canonical",
        }
    }
}
