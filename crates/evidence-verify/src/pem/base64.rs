//! Canonical decoding of the standard base64 alphabet with padding.

/// Decode `text`, or `None` when it is not canonical standard base64.
///
/// Canonical means a length that is a multiple of four, padding only in the
/// last one or two positions of the last group, and zero bits in the unused
/// low bits of a padded group, so every byte string has exactly one accepted
/// encoding.
pub(super) fn decode(text: &[u8]) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let groups = text.len() / 4;
    for (index, group) in text.as_chunks::<4>().0.iter().enumerate() {
        let padding = group
            .iter()
            .rev()
            .take_while(|&&symbol| symbol == b'=')
            .count();
        if padding > 2 || (padding > 0 && index + 1 != groups) {
            return None;
        }
        let mut value = 0u32;
        for &symbol in &group[..4 - padding] {
            value = (value << 6) | u32::from(sextet(symbol)?);
        }
        value <<= 6 * padding as u32;
        let bytes = [(value >> 16) as u8, (value >> 8) as u8, value as u8];
        let kept = 3 - padding;
        if bytes[kept..].iter().any(|&byte| byte != 0) {
            return None;
        }
        out.extend_from_slice(&bytes[..kept]);
    }
    Some(out)
}

fn sextet(symbol: u8) -> Option<u8> {
    match symbol {
        b'A'..=b'Z' => Some(symbol - b'A'),
        b'a'..=b'z' => Some(symbol - b'a' + 26),
        b'0'..=b'9' => Some(symbol - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_the_rfc_4648_test_vectors() {
        let vectors: [(&str, &str); 7] = [
            ("", ""),
            ("Zg==", "f"),
            ("Zm8=", "fo"),
            ("Zm9v", "foo"),
            ("Zm9vYg==", "foob"),
            ("Zm9vYmE=", "fooba"),
            ("Zm9vYmFy", "foobar"),
        ];
        for (encoded, plain) in vectors {
            assert_eq!(
                decode(encoded.as_bytes()).as_deref(),
                Some(plain.as_bytes()),
                "{encoded}"
            );
        }
    }

    #[test]
    fn decodes_every_alphabet_symbol() {
        let text = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bytes = decode(text).expect("alphabet decodes");
        let mut bits = 0u32;
        let mut count = 0u32;
        let mut symbols = Vec::new();
        for byte in bytes {
            bits = (bits << 8) | u32::from(byte);
            count += 8;
            while count >= 6 {
                count -= 6;
                symbols.push(((bits >> count) & 0x3f) as u8);
            }
        }
        assert_eq!(symbols, (0u8..64).collect::<Vec<_>>());
    }

    #[test]
    fn rejects_lengths_that_are_not_a_multiple_of_four() {
        for text in ["Z", "Zg", "Zg=", "Zm9vY", "Zm9vYmFy="] {
            assert_eq!(decode(text.as_bytes()), None, "{text}");
        }
    }

    #[test]
    fn rejects_symbols_outside_the_standard_alphabet() {
        for text in ["Zm9-", "Zm9_", "Zm 9", "Zm9\n", "Zm9.", "Zm9\u{7f}"] {
            assert_eq!(decode(text.as_bytes()), None, "{text:?}");
        }
        assert_eq!(decode(&[b'Z', b'm', b'9', 0xc3]), None);
    }

    #[test]
    fn rejects_misplaced_padding() {
        for text in [
            "====", "Z===", "=Zm9", "Z=m9", "Zg==Zm9v", "Zm8=Zm9v", "Zg=A",
        ] {
            assert_eq!(decode(text.as_bytes()), None, "{text}");
        }
    }

    #[test]
    fn rejects_non_zero_trailing_bits() {
        assert_eq!(decode(b"Zh=="), None);
        assert_eq!(decode(b"Zm9="), None);
        assert_eq!(decode(b"Zm8="), Some(b"fo".to_vec()));
    }
}
