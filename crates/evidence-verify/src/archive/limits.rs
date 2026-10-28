//! Entry name rules and size limits of admission, from the Admission section
//! of docs/adr/0085-in-browser-evidence-validation.md.

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;

/// Most entries a package may hold.
pub(super) const MAX_ENTRIES: usize = 32;

/// Longest entry name, in bytes.
pub(super) const MAX_NAME_LEN: usize = 128;

/// Largest sum of uncompressed entry sizes.
pub(super) const MAX_TOTAL: u64 = 25 * MIB;

/// Limit for the document. No entry may exceed it, so it is also the cap
/// applied to each entry while the archive is walked.
pub(super) const DOCUMENT_LIMIT: u64 = 16 * MIB;

/// Limit for each certificate revocation list.
const CRL_LIMIT: u64 = MIB;

/// Limit for `individual-signing.json`, whose integer arrays expand the
/// binding CRL.
const INDIVIDUAL_STATEMENT_LIMIT: u64 = 4 * MIB + MIB / 2;

/// Limit for every other entry, with a fixed name or without one.
const OTHER_LIMIT: u64 = 64 * KIB;

/// Names fixed in every package profile, which can never be the document.
const NAMES_FIXED_IN_EVERY_PROFILE: [&str; 5] = [
    "certificado.pem",
    "ca.pem",
    "crl.pem",
    "tsa-chain.pem",
    "INSTRUCCIONES.md",
];

/// Largest uncompressed size admitted for the entry `name` of an archive
/// whose entry names are `names`; names are compared byte for byte.
///
/// The document is not known before a profile is selected, so any name
/// that has both a `.sig` and a `.tsr` sibling may hold a document, unless
/// it is fixed in every profile. A server seal document may carry a name
/// that only the individual signing profiles fix, so such a candidate takes
/// the document limit instead of the fixed one. Every other entry without a
/// fixed limit, including the document's signature and response, takes the
/// 64 KiB limit.
pub(super) fn entry_limit(name: &[u8], names: &[&[u8]]) -> u64 {
    let fixed_everywhere = NAMES_FIXED_IN_EVERY_PROFILE
        .iter()
        .any(|fixed| fixed.as_bytes() == name);
    if name == b"crl.pem" {
        CRL_LIMIT
    } else if fixed_everywhere {
        OTHER_LIMIT
    } else if is_document_candidate(name, names) {
        DOCUMENT_LIMIT
    } else if name == b"binding-crl.der" || name == b"tsa-crl.pem" {
        CRL_LIMIT
    } else if name == b"individual-signing.json" {
        INDIVIDUAL_STATEMENT_LIMIT
    } else {
        OTHER_LIMIT
    }
}

/// True when `names` holds both `{name}.sig` and `{name}.tsr`.
fn is_document_candidate(name: &[u8], names: &[&[u8]]) -> bool {
    let has = |suffix: &[u8]| {
        names.iter().any(|other| {
            other.len() == name.len() + suffix.len()
                && other.starts_with(name)
                && other.ends_with(suffix)
        })
    };
    has(b".sig") && has(b".tsr")
}

/// True when `name` is 1 to [`MAX_NAME_LEN`] bytes of ASCII letters, digits,
/// dot, dash or underscore, starting with a letter or digit. This is the set
/// the server's archive entries are restricted to.
pub(super) fn name_is_safe(name: &[u8]) -> bool {
    let Some(first) = name.first() else {
        return false;
    };
    name.len() <= MAX_NAME_LEN
        && first.is_ascii_alphanumeric()
        && name
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}
