//! Individual signing v2: individual signing v1 plus the evidence manifest,
//! its time-stamp token, the time-stamping issuer CRL `tsa-crl.pem` and the
//! acceptance receipt.
//!
//! The manifest component holds the manifest's own checks and the token
//! over its digest; the token's anchor, profile and validity join the
//! certificates component, and its time-stamping certificate's revocation
//! the revocation component, decided by `tsa-crl.pem`. The document token's
//! time-stamping certificate is checked against `crl.pem` and `tsa-crl.pem`
//! when each was issued by its issuer. The context component adds the order
//! of the tokens and the acceptance, the covered registration instant and
//! the receipt. Rationale: the sections "The evidence manifest", "Revocation
//! outcomes" and "Instants and consistency" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

mod coverage;

use super::individual::{self, manifest_context};
use super::revocation::{self, TsaList};
use super::token::{self, Candidates};
use super::{current, in_entry, Checked, Evidence};
use crate::certificate;
use crate::crl::Crl;
use crate::manifest::{self, ENTRY, TOKEN_ENTRY, TSA_CRL_ENTRY};
use crate::receipt;
use crate::report::{Instant, InstantSource, TrustNote};
use crate::statements::uuid_text;

pub(super) fn check(evidence: &Evidence<'_>) -> Checked {
    let mut parts = individual::assemble(evidence);
    let bytes = evidence.entry(ENTRY);
    let digest = certificate::sha256(bytes);
    let token = token::check(
        evidence.entry(TOKEN_ENTRY),
        &digest,
        &Candidates {
            chain_pem: evidence.archive.get("tsa-chain.pem"),
            issuer: parts.root.as_ref().ok(),
        },
        evidence.policy,
    );
    let parsed = manifest::parse(bytes).map_err(|finding| in_entry(ENTRY, finding));
    let consent = parts.statements.as_ref().map(|(_, consent)| consent);

    let mut findings = match &parsed {
        Ok(manifest) => coverage::check(evidence, manifest, consent),
        Err(finding) => vec![finding.clone()],
    };
    findings.extend(
        token
            .findings
            .iter()
            .map(|finding| in_entry(TOKEN_ENTRY, finding.clone())),
    );
    parts.certificates.extend(
        token
            .chain
            .iter()
            .map(|finding| in_entry(TOKEN_ENTRY, finding.clone())),
    );

    let tsa_crl = super::pem_crl(TSA_CRL_ENTRY, evidence.entry(TSA_CRL_ENTRY));
    if let Err(finding) = &tsa_crl {
        parts.revocation.push(finding.clone());
    }
    let crl = parts.crl.as_ref().ok();
    let tsa_list = tsa_crl.as_ref().ok();
    let issuer_list = || crl.map(|list| TsaList::IfIssuer("crl.pem", list));
    let document_lists: Vec<TsaList<'_>> = issuer_list()
        .into_iter()
        .chain(tsa_list.map(|list| TsaList::IfIssuer(TSA_CRL_ENTRY, list)))
        .collect();
    let manifest_lists: Vec<TsaList<'_>> = tsa_list
        .map(|list| TsaList::Designated(TSA_CRL_ENTRY, list))
        .into_iter()
        .chain(issuer_list())
        .collect();
    let document_tsa = revocation::tsa_status(
        "document token time-stamping certificate",
        parts.token.tsa.as_ref(),
        parts.token.facts.as_ref(),
        &document_lists,
    );
    let manifest_tsa = revocation::tsa_status(
        "manifest token time-stamping certificate",
        token.tsa.as_ref(),
        token.facts.as_ref(),
        &manifest_lists,
    );

    let operation_id = match (&parsed, consent) {
        (Ok(manifest), _) => Some(manifest.operation_id.clone()),
        (Err(_), Some(consent)) => Some(uuid_text(consent.operation)),
        (Err(_), None) => None,
    };
    let (context, receipt) = manifest_context::check(&manifest_context::Input {
        record: parts.record.as_ref().ok(),
        document_token: parts.token.facts.as_ref(),
        manifest_token: token.facts.as_ref(),
        manifest: parsed.as_ref().ok(),
        operation_id,
        manifest_sha256: digest,
        receipt: evidence.entry(receipt::ENTRY),
    });

    let mut instants = std::mem::take(&mut parts.instants);
    if let Some(facts) = &token.facts {
        instants.push(Instant {
            label: "manifest token genTime".to_owned(),
            unix: facts.gen_time,
            source: InstantSource::Token,
        });
    }
    if let Some(receipt) = &receipt {
        instants.push(Instant {
            label: "receipt accepted_at".to_owned(),
            unix: receipt.accepted_at_seconds,
            source: InstantSource::ApplicationRecord,
        });
    }

    let mut subjects = parts.subjects();
    current::tsa_subjects(token.tsa.as_ref(), &mut subjects);
    let mut package_crls: Vec<&Crl> = parts.package_crls();
    package_crls.extend(tsa_crl.as_ref().ok());
    let current = current::status(&subjects, &package_crls, evidence.policy);

    let mut trust = std::mem::take(&mut parts.token.notes);
    merge_notes(&mut trust, token.notes);
    parts.revocation.push(document_tsa);
    parts.revocation.push(manifest_tsa);
    parts.context.extend(context);
    Checked {
        components: parts.into_components(Some(findings)),
        instants,
        trust,
        current,
    }
}

/// Add the notes of `more` that `notes` does not already hold, so a root
/// that issued both tokens is noted once.
fn merge_notes(notes: &mut Vec<TrustNote>, more: Vec<TrustNote>) {
    for note in more {
        if !notes.contains(&note) {
            notes.push(note);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::merge_notes;
    use crate::report::TrustNote;

    #[test]
    fn notes_about_the_same_root_are_kept_once() {
        let note = |fingerprint: &str| TrustNote {
            code: "trust.token_anchor_not_pinned",
            purpose: None,
            fingerprint: fingerprint.to_owned(),
            detail: String::new(),
        };
        let mut notes = vec![note("aa")];
        merge_notes(&mut notes, vec![note("aa"), note("bb")]);
        assert_eq!(notes, vec![note("aa"), note("bb")]);
    }
}
