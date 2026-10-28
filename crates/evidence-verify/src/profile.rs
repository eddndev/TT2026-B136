//! Recognition of the package profile from the inventory of entry names.
//!
//! Each profile fixes some entry names, required or optional, and admits one
//! document: the single name, other than the fixed names of that profile,
//! that has both a `.sig` and a `.tsr` sibling. Fixed names are evaluated per
//! profile, so a server seal document may carry a name that only individual
//! signing fixes. `INSTRUCCIONES.md` is optional in every profile and never
//! interpreted. Names are compared byte for byte; entry order is irrelevant.
//! A missing required entry, an unknown entry or a missing or ambiguous
//! document makes the package not supported. Rationale: the section
//! "Package profiles" of `docs/adr/0085-in-browser-evidence-validation.md`.

#[cfg(test)]
mod tests;

use crate::report::{Finding, Profile};

/// The auxiliary entry every profile admits and none interprets.
pub const INSTRUCTIONS: &str = "INSTRUCCIONES.md";

const SERVER_SEAL_V1: [&str; 3] = ["certificado.pem", "ca.pem", "crl.pem"];

const INDIVIDUAL_SIGNING_V1: [&str; 11] = [
    "certificado.pem",
    "ca.pem",
    "crl.pem",
    "tsa-chain.pem",
    "document-consent.bin",
    "document-consent.sig",
    "certificate-binding.bin",
    "certificate-binding.sig",
    "binding-root.der",
    "binding-crl.der",
    "individual-signing.json",
];

const INDIVIDUAL_SIGNING_V2: [&str; 15] = [
    "certificado.pem",
    "ca.pem",
    "crl.pem",
    "tsa-chain.pem",
    "document-consent.bin",
    "document-consent.sig",
    "certificate-binding.bin",
    "certificate-binding.sig",
    "binding-root.der",
    "binding-crl.der",
    "individual-signing.json",
    "manifiesto.json",
    "manifiesto.json.tsr",
    "tsa-crl.pem",
    "recibo-aceptacion.json",
];

/// The fixed names of one profile.
struct Spec {
    profile: Profile,
    required: &'static [&'static str],
    optional: &'static [&'static str],
}

impl Spec {
    fn fixes(&self, name: &str) -> bool {
        name == INSTRUCTIONS || self.required.contains(&name) || self.optional.contains(&name)
    }
}

/// Every known profile, simplest first; a new profile is one more entry.
const SPECS: [Spec; 3] = [
    Spec {
        profile: Profile::ServerSealV1,
        required: &SERVER_SEAL_V1,
        optional: &["tsa-chain.pem"],
    },
    Spec {
        profile: Profile::IndividualSigningV1,
        required: &INDIVIDUAL_SIGNING_V1,
        optional: &[],
    },
    Spec {
        profile: Profile::IndividualSigningV2,
        required: &INDIVIDUAL_SIGNING_V2,
        optional: &[],
    },
];

/// The recognized profile and the name of its document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub profile: Profile,
    pub document: String,
}

impl Layout {
    /// The entry holding the detached signature of the document.
    pub fn signature(&self) -> String {
        format!("{}.sig", self.document)
    }

    /// The entry holding the time-stamp response over the document.
    pub fn token(&self) -> String {
        format!("{}.tsr", self.document)
    }
}

/// Select the one profile whose inventory `names` has exactly.
///
/// When none matches, the finding explains the mismatch against the profile
/// with the most fixed names present, the simplest one on a tie.
pub fn recognize(names: &[&str]) -> Result<Layout, Finding> {
    let mut matched = Vec::new();
    let mut closest: Option<(usize, Finding)> = None;
    for spec in &SPECS {
        match evaluate(spec, names) {
            Ok(document) => matched.push(Layout {
                profile: spec.profile,
                document,
            }),
            Err(finding) => {
                let present = names
                    .iter()
                    .filter(|name| **name != INSTRUCTIONS && spec.fixes(name))
                    .count();
                if closest.as_ref().is_none_or(|(best, _)| present > *best) {
                    closest = Some((present, finding));
                }
            }
        }
    }
    match matched.len() {
        1 => Ok(matched.remove(0)),
        0 => Err(closest
            .map(|(_, finding)| finding)
            .unwrap_or_else(|| Finding::not_supported("profile.unrecognized", ""))),
        _ => Err(Finding::not_supported(
            "profile.ambiguous",
            "the inventory matches several profiles",
        )),
    }
}

/// The document name of `names` under `spec`, or why `names` is not that
/// profile's inventory.
fn evaluate(spec: &Spec, names: &[&str]) -> Result<String, Finding> {
    let missing: Vec<&str> = spec
        .required
        .iter()
        .copied()
        .filter(|required| !names.contains(required))
        .collect();
    if !missing.is_empty() {
        return Err(Finding::not_supported(
            "profile.missing_entry",
            missing.join(", "),
        ));
    }
    let free: Vec<&str> = names
        .iter()
        .copied()
        .filter(|name| !spec.fixes(name))
        .collect();
    let has = |name: String| free.contains(&name.as_str());
    let documents: Vec<&str> = free
        .iter()
        .copied()
        .filter(|name| has(format!("{name}.sig")) && has(format!("{name}.tsr")))
        .collect();
    let document = match documents.as_slice() {
        [only] => *only,
        [] => {
            return Err(Finding::not_supported(
                "profile.no_document",
                "no entry has both a .sig and a .tsr sibling",
            ))
        }
        several => {
            return Err(Finding::not_supported(
                "profile.ambiguous_document",
                several.join(", "),
            ))
        }
    };
    let signature = format!("{document}.sig");
    let token = format!("{document}.tsr");
    let unknown: Vec<&str> = free
        .iter()
        .copied()
        .filter(|name| *name != document && *name != signature && *name != token)
        .collect();
    if !unknown.is_empty() {
        return Err(Finding::not_supported(
            "profile.unknown_entry",
            unknown.join(", "),
        ));
    }
    Ok(document.to_owned())
}
