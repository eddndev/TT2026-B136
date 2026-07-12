//! Outbound port for building flat evidence archives.
//!
//! An evidence archive bundles a document with the artifacts a third
//! party needs to verify it independently. The domain fixes only the
//! shape: named entries with byte content, no directories. The container
//! format is the adapter's concern.

use crate::error::DomainError;

/// Longest accepted entry name, in bytes.
pub const MAX_ENTRY_NAME_LEN: usize = 128;

/// One named member of an evidence archive.
///
/// Names are restricted to a conservative ASCII set so they stay usable
/// verbatim in file systems, shell command lines, and the verification
/// instructions shipped inside the archive: letters, digits, dot, dash,
/// and underscore, starting with a letter or digit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    name: String,
    content: Vec<u8>,
}

impl ArchiveEntry {
    /// Builds an entry, rejecting names outside the safe set.
    pub fn new(name: impl Into<String>, content: Vec<u8>) -> Result<Self, DomainError> {
        let name = name.into();
        if !name_is_safe(&name) {
            return Err(DomainError::InvalidArchiveEntryName { name });
        }
        Ok(Self { name, content })
    }

    /// The entry's file name inside the archive.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The entry's byte content.
    pub fn content(&self) -> &[u8] {
        &self.content
    }
}

/// True when the name fits the safe set: 1 to [`MAX_ENTRY_NAME_LEN`]
/// bytes of ASCII letters, digits, dot, dash, or underscore, starting
/// with a letter or digit. This excludes path separators, so a name can
/// never escape the directory an archive is extracted into.
fn name_is_safe(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_ENTRY_NAME_LEN {
        return false;
    }
    if !bytes[0].is_ascii_alphanumeric() {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// Outbound port: encodes named entries into one archive byte stream.
pub trait ArchiveWriter {
    /// Encodes `entries` in order into a single archive.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::ArchiveWriteFailure`] when the entries
    /// cannot be represented in the adapter's format (for example
    /// duplicate names or sizes beyond its limits).
    fn write_archive(&self, entries: &[ArchiveEntry]) -> Result<Vec<u8>, DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_expose_their_name_and_content() {
        let entry = ArchiveEntry::new("document.txt", vec![1, 2, 3]).unwrap();
        assert_eq!(entry.name(), "document.txt");
        assert_eq!(entry.content(), &[1, 2, 3]);
    }

    #[test]
    fn names_with_path_separators_are_rejected() {
        for name in ["a/b.txt", "a\\b.txt", "../escape", "dir/"] {
            let err = ArchiveEntry::new(name, Vec::new()).unwrap_err();
            assert_eq!(
                err,
                DomainError::InvalidArchiveEntryName {
                    name: name.to_string(),
                },
                "name {name:?} must be rejected"
            );
        }
    }

    #[test]
    fn names_must_start_with_a_letter_or_digit() {
        for name in ["", ".hidden", "-flag.txt", "_x"] {
            assert!(
                ArchiveEntry::new(name, Vec::new()).is_err(),
                "name {name:?} must be rejected"
            );
        }
    }

    #[test]
    fn names_outside_ascii_or_too_long_are_rejected() {
        assert!(ArchiveEntry::new("con espacio.txt", Vec::new()).is_err());
        assert!(ArchiveEntry::new("acta\u{00f3}.txt", Vec::new()).is_err());
        let long = "a".repeat(MAX_ENTRY_NAME_LEN + 1);
        assert!(ArchiveEntry::new(long, Vec::new()).is_err());
    }

    #[test]
    fn ordinary_file_names_are_accepted() {
        for name in ["INSTRUCCIONES.md", "doc-1.txt.sig", "ca_root.pem", "1.tsr"] {
            assert!(
                ArchiveEntry::new(name, Vec::new()).is_ok(),
                "name {name:?} must be accepted"
            );
        }
    }

    #[test]
    fn entry_name_error_displays_the_offending_name() {
        let err = DomainError::InvalidArchiveEntryName {
            name: "a/b".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "archive entry name is not usable: \"a/b\" (ascii letters, digits, \
             dot, dash, and underscore only, starting with a letter or digit)"
        );
    }

    /// Test double proving the port is object safe; it is not a real
    /// archive format.
    struct Concatenator;

    impl ArchiveWriter for Concatenator {
        fn write_archive(&self, entries: &[ArchiveEntry]) -> Result<Vec<u8>, DomainError> {
            if entries.is_empty() {
                return Err(DomainError::ArchiveWriteFailure("no entries".to_string()));
            }
            Ok(entries
                .iter()
                .flat_map(|entry| entry.content().to_vec())
                .collect())
        }
    }

    #[test]
    fn port_is_object_safe_and_surfaces_failures() {
        let writer: &dyn ArchiveWriter = &Concatenator;
        let entry = ArchiveEntry::new("a.bin", vec![1]).unwrap();
        assert_eq!(writer.write_archive(&[entry]).unwrap(), vec![1]);
        let err = writer.write_archive(&[]).unwrap_err();
        assert_eq!(err.to_string(), "archive writing failed: no entries");
    }
}
