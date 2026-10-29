//! The exact schema of a parsed manifest value.
//!
//! Every object must have exactly its member set; fixed members must hold
//! their only admitted value; digests are 64 lowercase hexadecimal digits;
//! identifiers are lowercase hyphenated UUIDs; artifacts are sorted by entry
//! name in byte order with unique entries and unique roles; the only signer
//! is `signer-1`. Details name the offending member by its path, never its
//! value.

use super::{
    Artifact, Manifest, Role, Signer, SignerEntries, ALGORITHMS, PROFILE, PROFILE_VERSION, PURPOSE,
    SIGNER_ID,
};
use crate::json::Value;
use crate::report::Finding;

const TOP: [&str; 12] = [
    "profile",
    "profile_version",
    "purpose",
    "deployment_id",
    "case_id",
    "document_id",
    "document_version",
    "document_sha256",
    "operation_id",
    "algorithms",
    "artifacts",
    "signers",
];
const ARTIFACT: [&str; 4] = ["entry", "role", "sha256", "size"];
const SIGNER: [&str; 5] = [
    "id",
    "account_id",
    "binding_id",
    "binding_registered_at",
    "entries",
];

/// Code of every rejection of the manifest schema.
const SCHEMA: &str = "manifest.schema";

fn schema(detail: impl Into<String>) -> Finding {
    Finding::failed(SCHEMA, detail)
}

/// `text`, chosen by whoever wrote the entry, quoted and escaped to ASCII
/// so that it can neither pose as report text nor reorder the detail.
pub(crate) fn shown(text: &str) -> String {
    format!("\"{}\"", text.escape_default())
}

/// The members of an object that has exactly `keys`.
pub(crate) struct Fields<'a> {
    members: &'a [(String, Value)],
    path: String,
    /// The code of every schema rejection.
    code: &'static str,
}

impl<'a> Fields<'a> {
    /// The members of `value` at `path`, rejecting with `code` an object
    /// whose member set is not exactly `keys`.
    pub(crate) fn of(
        value: &'a Value,
        keys: &[&str],
        path: &str,
        code: &'static str,
    ) -> Result<Self, Finding> {
        let schema = |detail: String| Finding::failed(code, detail);
        let members = value
            .as_object()
            .ok_or_else(|| schema(format!("{path} is not an object")))?;
        if let Some((key, _)) = members
            .iter()
            .find(|(key, _)| !keys.contains(&key.as_str()))
        {
            return Err(schema(format!("{path} has the member {}", shown(key))));
        }
        if let Some(key) = keys
            .iter()
            .find(|key| !members.iter().any(|(name, _)| name == *key))
        {
            return Err(schema(format!("{path} lacks the member {key:?}")));
        }
        Ok(Self {
            members,
            path: path.to_owned(),
            code,
        })
    }

    fn fail(&self, detail: String) -> Finding {
        Finding::failed(self.code, detail)
    }

    /// A nested object with exactly `keys`, rejected with the same code.
    pub(crate) fn object(&self, key: &str, keys: &[&str]) -> Result<Fields<'a>, Finding> {
        Fields::of(self.get(key), keys, &self.path(key), self.code)
    }

    fn path(&self, key: &str) -> String {
        format!("{}/{key}", self.path)
    }

    pub(crate) fn get(&self, key: &str) -> &'a Value {
        self.members
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
            .expect("member presence checked in Fields::of")
    }

    pub(crate) fn text(&self, key: &str) -> Result<&'a str, Finding> {
        self.get(key)
            .as_str()
            .ok_or_else(|| self.fail(format!("{} is not a string", self.path(key))))
    }

    pub(crate) fn fixed(&self, key: &str, expected: &str) -> Result<(), Finding> {
        if self.text(key)? == expected {
            Ok(())
        } else {
            Err(self.fail(format!("{} is not {expected:?}", self.path(key))))
        }
    }

    /// A non-negative integer of at least `minimum`; the canonical parse
    /// already bounds it by 2^53 - 1.
    pub(crate) fn integer(&self, key: &str, minimum: u64) -> Result<u64, Finding> {
        self.get(key)
            .as_integer()
            .and_then(|number| u64::try_from(number).ok())
            .filter(|number| *number >= minimum)
            .ok_or_else(|| {
                self.fail(format!(
                    "{} is not an integer of at least {minimum}",
                    self.path(key)
                ))
            })
    }

    pub(crate) fn uuid(&self, key: &str) -> Result<String, Finding> {
        let text = self.text(key)?;
        let shaped = text.len() == 36
            && text.bytes().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                _ => matches!(byte, b'0'..=b'9' | b'a'..=b'f'),
            });
        if shaped {
            Ok(text.to_owned())
        } else {
            Err(self.fail(format!("{} is not a lowercase UUID", self.path(key))))
        }
    }

    pub(crate) fn digest(&self, key: &str) -> Result<[u8; 32], Finding> {
        let text = self.text(key)?;
        let invalid = || self.fail(format!("{} is not a lowercase SHA-256", self.path(key)));
        if text.len() != 64 {
            return Err(invalid());
        }
        let mut digest = [0u8; 32];
        for (index, pair) in text.as_bytes().chunks(2).enumerate() {
            let nibble = |byte: u8| match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                _ => None,
            };
            let (Some(high), Some(low)) = (nibble(pair[0]), nibble(pair[1])) else {
                return Err(invalid());
            };
            digest[index] = high << 4 | low;
        }
        Ok(digest)
    }

    pub(crate) fn array(&self, key: &str) -> Result<&'a [Value], Finding> {
        self.get(key)
            .as_array()
            .ok_or_else(|| self.fail(format!("{} is not an array", self.path(key))))
    }
}

pub(super) fn manifest(value: &Value) -> Result<Manifest, Finding> {
    let top = Fields::of(value, &TOP, "manifest", SCHEMA)?;
    top.fixed("profile", PROFILE)?;
    if top.integer("profile_version", 0)? != PROFILE_VERSION {
        return Err(schema("manifest/profile_version is not 1"));
    }
    top.fixed("purpose", PURPOSE)?;
    let keys: Vec<&str> = ALGORITHMS.iter().map(|(key, _)| *key).collect();
    let algorithms = top.object("algorithms", &keys)?;
    for (key, expected) in ALGORITHMS {
        algorithms.fixed(key, expected)?;
    }
    // Only a manifest of this exact profile, version, purpose and
    // algorithms is not supported for having more signers; any other is
    // failed, so a second signer never hides a wrong fixed member.
    let signers = top.array("signers")?;
    if signers.len() > 1 {
        return Err(Finding::not_supported(
            "manifest.signers_not_supported",
            format!("{} signers; this profile admits one", signers.len()),
        ));
    }
    let artifacts = artifacts(top.array("artifacts")?)?;
    let [signer] = signers else {
        return Err(schema("manifest/signers is empty"));
    };
    Ok(Manifest {
        deployment_id: top.uuid("deployment_id")?,
        case_id: top.uuid("case_id")?,
        document_id: top.uuid("document_id")?,
        document_version: top.integer("document_version", 1)?,
        document_sha256: top.digest("document_sha256")?,
        operation_id: top.uuid("operation_id")?,
        artifacts,
        signers: vec![signer_of(signer)?],
    })
}

fn artifacts(items: &[Value]) -> Result<Vec<Artifact>, Finding> {
    let mut parsed: Vec<Artifact> = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let path = format!("manifest/artifacts/{index}");
        let fields = Fields::of(item, &ARTIFACT, &path, SCHEMA)?;
        let entry = fields.text("entry")?;
        if entry.is_empty() {
            return Err(schema(format!("{path}/entry is empty")));
        }
        if let Some(previous) = parsed.last() {
            match previous.entry.as_bytes().cmp(entry.as_bytes()) {
                core::cmp::Ordering::Equal => {
                    return Err(Finding::failed(
                        "manifest.duplicate_artifact",
                        format!("{path} repeats the previous entry"),
                    ))
                }
                core::cmp::Ordering::Greater => {
                    return Err(Finding::failed(
                        "manifest.artifacts_unsorted",
                        format!("{path} sorts before the previous entry"),
                    ))
                }
                core::cmp::Ordering::Less => {}
            }
        }
        let role = Role::parse(fields.text("role")?)
            .ok_or_else(|| schema(format!("{path}/role is not a known role")))?;
        if parsed.iter().any(|artifact| artifact.role == role) {
            return Err(schema(format!("{path}/role repeats {}", role.as_str())));
        }
        parsed.push(Artifact {
            entry: entry.to_owned(),
            role,
            sha256: fields.digest("sha256")?,
            size: fields.integer("size", 0)?,
        });
    }
    Ok(parsed)
}

fn signer_of(value: &Value) -> Result<Signer, Finding> {
    let path = "manifest/signers/0";
    let fields = Fields::of(value, &SIGNER, path, SCHEMA)?;
    fields.fixed("id", SIGNER_ID)?;
    let names = SignerEntries::for_document("")
        .members()
        .map(|(key, _, _)| key);
    let entries = fields.object("entries", &names)?;
    let entry = |key: &str| -> Result<String, Finding> {
        let text = entries.text(key)?;
        if text.is_empty() {
            return Err(schema(format!("{path}/entries/{key} is empty")));
        }
        Ok(text.to_owned())
    };
    Ok(Signer {
        id: SIGNER_ID.to_owned(),
        account_id: fields.uuid("account_id")?,
        binding_id: fields.uuid("binding_id")?,
        binding_registered_at: i64::try_from(fields.integer("binding_registered_at", 0)?)
            .map_err(|_| schema(format!("{path}/binding_registered_at is out of range")))?,
        entries: SignerEntries {
            document_signature: entry("document_signature")?,
            signer_certificate: entry("signer_certificate")?,
            binding_statement: entry("binding_statement")?,
            binding_signature: entry("binding_signature")?,
            binding_root: entry("binding_root")?,
            binding_crl: entry("binding_crl")?,
            consent_statement: entry("consent_statement")?,
            consent_signature: entry("consent_signature")?,
        },
    })
}
