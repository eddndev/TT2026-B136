//! Random generator for human-typeable recovery codes.

use domain::crypto::recovery::RecoveryCodeGenerator;
use domain::DomainError;
use zeroize::Zeroizing;

/// Crockford-style base32 alphabet: the digits and the upper-case letters
/// minus I, L, O, and U, which are easy to misread when typed back.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Characters drawn per code: ten characters of five bits each, 50 bits of
/// entropy.
const CODE_CHARS: usize = 10;

/// Characters per hyphen-separated group, giving codes the shape
/// "XXXXX-XXXXX".
const GROUP_CHARS: usize = 5;

/// Draws recovery codes from the operating system's random generator.
#[derive(Debug, Default, Clone, Copy)]
pub struct RandomRecoveryCodeGenerator;

impl RecoveryCodeGenerator for RandomRecoveryCodeGenerator {
    fn generate(&self, count: usize) -> Result<Vec<Zeroizing<String>>, DomainError> {
        (0..count).map(|_| generate_one()).collect()
    }
}

fn generate_one() -> Result<Zeroizing<String>, DomainError> {
    let mut raw = Zeroizing::new([0u8; CODE_CHARS]);
    getrandom::getrandom(raw.as_mut_slice())
        .map_err(|err| DomainError::RandomnessFailed(err.to_string()))?;
    let mut code = Zeroizing::new(String::with_capacity(CODE_CHARS + 1));
    for (index, byte) in raw.iter().enumerate() {
        if index > 0 && index % GROUP_CHARS == 0 {
            code.push('-');
        }
        // Keeping five bits of each random byte indexes the 32-character
        // alphabet uniformly, with no modulo bias.
        code.push(ALPHABET[usize::from(byte & 0x1f)] as char);
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::crypto::recovery::RECOVERY_CODE_COUNT;

    #[test]
    fn codes_have_the_grouped_human_typeable_shape() {
        let codes = RandomRecoveryCodeGenerator
            .generate(RECOVERY_CODE_COUNT)
            .unwrap();
        assert_eq!(codes.len(), RECOVERY_CODE_COUNT);
        for code in &codes {
            let (first, rest) = code.split_once('-').expect("one hyphen per code");
            assert_eq!(first.len(), GROUP_CHARS, "code: {}", code.as_str());
            assert_eq!(rest.len(), GROUP_CHARS, "code: {}", code.as_str());
            for ch in first.chars().chain(rest.chars()) {
                assert!(
                    ALPHABET.contains(&(ch as u8)),
                    "unexpected character {ch:?} in {}",
                    code.as_str()
                );
            }
        }
    }

    #[test]
    fn codes_within_a_batch_are_distinct() {
        let codes = RandomRecoveryCodeGenerator
            .generate(RECOVERY_CODE_COUNT)
            .unwrap();
        for (i, left) in codes.iter().enumerate() {
            for right in codes.iter().skip(i + 1) {
                assert_ne!(left.as_str(), right.as_str());
            }
        }
    }

    #[test]
    fn generating_zero_codes_yields_an_empty_batch() {
        assert!(RandomRecoveryCodeGenerator.generate(0).unwrap().is_empty());
    }
}
