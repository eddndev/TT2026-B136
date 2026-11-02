//! The exact schema of a parsed contract manifest value.
//!
//! Every object must have exactly its member set; fixed members must hold
//! their only admitted value; each artifact's entry must be the one its party
//! and role name; artifacts are sorted by entry name in byte order; there are
//! exactly two signers in position order whose fixed members follow from
//! their position. Details name the offending member by its path, never its
//! value.

use super::{
    side, signer_id, AccountRole, ContractArtifact, ContractManifest, ContractRole, ContractSigner,
    PARTY_ENTRIES, PROFILE_VERSION, PURPOSE,
};
use crate::json::Value;
use crate::manifest::schema::{shown, Fields};
use crate::manifest::{ALGORITHMS, PROFILE};
use crate::report::Finding;

const TOP: [&str; 14] = [
    "profile",
    "profile_version",
    "purpose",
    "deployment_id",
    "case_id",
    "request_id",
    "document_id",
    "document_version",
    "document_sha256",
    "request_sha256",
    "expires_at",
    "algorithms",
    "artifacts",
    "signers",
];
const ARTIFACT: [&str; 5] = ["entry", "party", "role", "sha256", "size"];
const SIGNER: [&str; 9] = [
    "id",
    "position",
    "side",
    "account_id",
    "account_role",
    "binding_id",
    "binding_registered_at",
    "operation_id",
    "entries",
];

/// Code of every rejection of the contract manifest schema.
const SCHEMA: &str = "manifest.contract_schema";

fn schema(detail: impl Into<String>) -> Finding {
    Finding::failed(SCHEMA, detail)
}

pub(super) fn manifest(value: &Value) -> Result<ContractManifest, Finding> {
    let top = Fields::of(value, &TOP, "manifest", SCHEMA)?;
    top.fixed("profile", PROFILE)?;
    if top.integer("profile_version", 0)? != PROFILE_VERSION {
        return Err(schema("manifest/profile_version is not 2"));
    }
    top.fixed("purpose", PURPOSE)?;
    let keys: Vec<&str> = ALGORITHMS.iter().map(|(key, _)| *key).collect();
    let algorithms = top.object("algorithms", &keys)?;
    for (key, expected) in ALGORITHMS {
        algorithms.fixed(key, expected)?;
    }
    let artifacts = artifacts(top.array("artifacts")?)?;
    let [client, firm] = top.array("signers")? else {
        return Err(schema("manifest/signers does not hold exactly two signers"));
    };
    Ok(ContractManifest {
        deployment_id: top.uuid("deployment_id")?,
        case_id: top.uuid("case_id")?,
        request_id: top.uuid("request_id")?,
        document_id: top.uuid("document_id")?,
        document_version: top.integer("document_version", 1)?,
        document_sha256: top.digest("document_sha256")?,
        request_sha256: top.digest("request_sha256")?,
        expires_at: seconds(&top, "manifest", "expires_at")?,
        artifacts,
        signers: [signer(client, 0)?, signer(firm, 1)?],
    })
}

fn seconds(fields: &Fields<'_>, path: &str, key: &str) -> Result<i64, Finding> {
    i64::try_from(fields.integer(key, 0)?)
        .map_err(|_| schema(format!("{path}/{key} is out of range")))
}

fn artifacts(items: &[Value]) -> Result<Vec<ContractArtifact>, Finding> {
    let mut parsed: Vec<ContractArtifact> = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let path = format!("manifest/artifacts/{index}");
        let fields = Fields::of(item, &ARTIFACT, &path, SCHEMA)?;
        let entry = fields.text("entry")?;
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
        let party = u8::try_from(fields.integer("party", 0)?)
            .ok()
            .filter(|party| *party <= 2)
            .ok_or_else(|| schema(format!("{path}/party is not 0, 1 or 2")))?;
        let role = ContractRole::parse(fields.text("role")?)
            .ok_or_else(|| schema(format!("{path}/role is not a known role")))?;
        if role.entry(party) != Some(entry) {
            return Err(schema(format!(
                "{path}/entry {} is not the entry of party {party} and role {}",
                shown(entry),
                role.as_str()
            )));
        }
        parsed.push(ContractArtifact {
            entry: entry.to_owned(),
            party,
            role,
            sha256: fields.digest("sha256")?,
            size: fields.integer("size", 0)?,
        });
    }
    Ok(parsed)
}

/// The signer at `index` of `signers`: index 0 is the Client at position 1,
/// index 1 the firm signer at position 2.
fn signer(value: &Value, index: usize) -> Result<ContractSigner, Finding> {
    let path = format!("manifest/signers/{index}");
    let fields = Fields::of(value, &SIGNER, &path, SCHEMA)?;
    fields.fixed("id", signer_id(index))?;
    if fields.integer("position", 0)? != index as u64 + 1 {
        return Err(schema(format!("{path}/position is not {}", index + 1)));
    }
    fields.fixed("side", side(index))?;
    let account_role = match (index, fields.text("account_role")?) {
        (0, "client") => AccountRole::Client,
        (1, "owner") => AccountRole::Owner,
        (1, "litigator") => AccountRole::Litigator,
        _ => {
            return Err(schema(format!(
                "{path}/account_role is not admitted at this position"
            )))
        }
    };
    let members = PARTY_ENTRIES[index].members();
    let names = members.map(|(key, _, _)| key);
    let entries = fields.object("entries", &names)?;
    for (key, _, entry) in members {
        entries.fixed(key, entry)?;
    }
    Ok(ContractSigner {
        account_id: fields.uuid("account_id")?,
        account_role,
        binding_id: fields.uuid("binding_id")?,
        binding_registered_at: seconds(&fields, &path, "binding_registered_at")?,
        operation_id: fields.uuid("operation_id")?,
    })
}
