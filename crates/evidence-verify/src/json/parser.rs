//! Bounded recursive-descent parser for the admitted JSON subset.

use std::collections::BTreeSet;

use super::{ErrorKind, JsonError, Limits, Value, MAX_DEPTH};

/// Parse one complete document, rejecting anything outside the subset.
///
/// Whitespace is limited to space, tab, line feed and carriage return; a
/// byte order mark is not whitespace.
pub fn parse(bytes: &[u8], limits: Limits) -> Result<Value, JsonError> {
    let text = core::str::from_utf8(bytes)
        .map_err(|error| JsonError::new(ErrorKind::InvalidUtf8, error.valid_up_to()))?;
    let mut parser = Parser {
        text,
        bytes,
        position: 0,
        values: 0,
        limits,
    };
    parser.skip_whitespace();
    let value = parser.value(0)?;
    parser.skip_whitespace();
    if parser.position != bytes.len() {
        return Err(parser.error(ErrorKind::TrailingData));
    }
    Ok(value)
}

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    position: usize,
    values: usize,
    limits: Limits,
}

impl Parser<'_> {
    fn error(&self, kind: ErrorKind) -> JsonError {
        JsonError::new(kind, self.position)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn next_byte(&mut self) -> Result<u8, JsonError> {
        let byte = self
            .peek()
            .ok_or_else(|| self.error(ErrorKind::UnexpectedEnd))?;
        self.position += 1;
        Ok(byte)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.position += 1;
        }
    }

    fn expect(&mut self, wanted: u8) -> Result<(), JsonError> {
        match self.peek() {
            Some(byte) if byte == wanted => {
                self.position += 1;
                Ok(())
            }
            Some(_) => Err(self.error(ErrorKind::UnexpectedByte)),
            None => Err(self.error(ErrorKind::UnexpectedEnd)),
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, JsonError> {
        if self.values >= self.limits.max_values {
            return Err(self.error(ErrorKind::TooManyValues));
        }
        self.values += 1;
        match self.peek() {
            None => Err(self.error(ErrorKind::UnexpectedEnd)),
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => self.string().map(Value::String),
            Some(b'-' | b'0'..=b'9') => self.integer().map(Value::Integer),
            Some(b't' | b'f' | b'n') => self.literal(),
            Some(_) => Err(self.error(ErrorKind::UnexpectedByte)),
        }
    }

    fn literal(&self) -> Result<Value, JsonError> {
        let rest = &self.bytes[self.position..];
        if [&b"true"[..], b"false", b"null"]
            .iter()
            .any(|word| rest.starts_with(word))
        {
            Err(self.error(ErrorKind::Literal))
        } else {
            Err(self.error(ErrorKind::UnexpectedByte))
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        self.position += 1;
        let mut members: Vec<(String, Value)> = Vec::new();
        // An ordered set keeps duplicate detection logarithmic per key, so a
        // wide hostile object cannot make parsing quadratic, and it needs no
        // random hashing seed.
        let mut keys: BTreeSet<String> = BTreeSet::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.skip_whitespace();
            let key_offset = self.position;
            if self.peek() != Some(b'"') {
                return Err(self.error(if self.peek().is_none() {
                    ErrorKind::UnexpectedEnd
                } else {
                    ErrorKind::UnexpectedByte
                }));
            }
            let key = self.string()?;
            if !keys.insert(key.clone()) {
                return Err(JsonError::new(ErrorKind::DuplicateKey, key_offset));
            }
            self.skip_whitespace();
            self.expect(b':')?;
            self.skip_whitespace();
            let value = self.value(depth)?;
            members.push((key, value));
            self.skip_whitespace();
            match self.next_byte()? {
                b',' => continue,
                b'}' => return Ok(Value::Object(members)),
                _ => {
                    self.position -= 1;
                    return Err(self.error(ErrorKind::UnexpectedByte));
                }
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.error(ErrorKind::TooDeep));
        }
        self.position += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.value(depth)?);
            self.skip_whitespace();
            match self.next_byte()? {
                b',' => continue,
                b']' => return Ok(Value::Array(items)),
                _ => {
                    self.position -= 1;
                    return Err(self.error(ErrorKind::UnexpectedByte));
                }
            }
        }
    }

    /// An integer: optional minus, then `0` or a non-zero digit followed by
    /// digits, with no fraction or exponent.
    fn integer(&mut self) -> Result<i128, JsonError> {
        let start = self.position;
        let negative = self.peek() == Some(b'-');
        if negative {
            self.position += 1;
        }
        let first = self.next_byte()?;
        if !first.is_ascii_digit() {
            self.position -= 1;
            return Err(self.error(ErrorKind::UnexpectedByte));
        }
        if first == b'0' && matches!(self.peek(), Some(b'0'..=b'9')) {
            return Err(JsonError::new(ErrorKind::LeadingZero, start));
        }
        let mut magnitude = i128::from(first - b'0');
        while let Some(digit @ b'0'..=b'9') = self.peek() {
            magnitude = magnitude
                .checked_mul(10)
                .and_then(|value| value.checked_add(i128::from(digit - b'0')))
                .ok_or_else(|| JsonError::new(ErrorKind::IntegerOutOfRange, start))?;
            self.position += 1;
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(self.error(ErrorKind::NotInteger));
        }
        let value = if negative { -magnitude } else { magnitude };
        if negative && magnitude == 0 {
            return Err(JsonError::new(ErrorKind::NegativeZero, start));
        }
        if value < i128::from(i64::MIN) || value > i128::from(u64::MAX) {
            return Err(JsonError::new(ErrorKind::IntegerOutOfRange, start));
        }
        Ok(value)
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.position += 1;
        let mut decoded = String::new();
        loop {
            let run_start = self.position;
            while let Some(byte) = self.peek() {
                if byte == b'"' || byte == b'\\' || byte < 0x20 {
                    break;
                }
                self.position += 1;
            }
            // Stops happen only at ASCII bytes, so both ends are boundaries.
            decoded.push_str(&self.text[run_start..self.position]);
            match self.next_byte()? {
                b'"' => return Ok(decoded),
                b'\\' => decoded.push(self.escape()?),
                _ => {
                    self.position -= 1;
                    return Err(self.error(ErrorKind::ControlCharacter));
                }
            }
        }
    }

    fn escape(&mut self) -> Result<char, JsonError> {
        let start = self.position - 1;
        let invalid = JsonError::new(ErrorKind::InvalidEscape, start);
        let unpaired = JsonError::new(ErrorKind::UnpairedSurrogate, start);
        let decoded = match self.next_byte().map_err(|_| invalid)? {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => {
                let unit = self.hex4().ok_or(invalid)?;
                match unit {
                    0xd800..=0xdbff => {
                        if !self.bytes[self.position..].starts_with(b"\\u") {
                            return Err(unpaired);
                        }
                        self.position += 2;
                        let low = self.hex4().ok_or(invalid)?;
                        if !(0xdc00..=0xdfff).contains(&low) {
                            return Err(unpaired);
                        }
                        let scalar = 0x10000 + ((unit - 0xd800) << 10) + (low - 0xdc00);
                        char::from_u32(scalar).ok_or(unpaired)?
                    }
                    0xdc00..=0xdfff => return Err(unpaired),
                    other => char::from_u32(other).ok_or(invalid)?,
                }
            }
            _ => return Err(invalid),
        };
        Ok(decoded)
    }

    fn hex4(&mut self) -> Option<u32> {
        let digits = self.bytes.get(self.position..self.position + 4)?;
        let mut unit = 0u32;
        for digit in digits {
            unit = unit * 16 + char::from(*digit).to_digit(16)?;
        }
        self.position += 4;
        Some(unit)
    }
}
