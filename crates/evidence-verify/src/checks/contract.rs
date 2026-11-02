//! Bilateral contract signing: the full package, with both party statements,
//! Td, the manifest and Tf, and the partial package the Client keeps after
//! signing, with the position 1 statement and T1 only.
//!
//! The Client is judged at T1 and the firm signer at Tf, through the token
//! interval, never through an application record. T1 and its time-stamping
//! certificate are judged with `parte-1-crl.pem`, Td and Tf with
//! `parte-2-crl.pem`, so a CRL renewal between the two signatures never
//! leaves T1 without a covering list. In the full package each binding is
//! judged at the registration instant the manifest states; in the partial
//! package, which has no covered registration instant, at T1. The partial
//! package always carries an indeterminate `contract.partial`. The receipts
//! are parsed for their schema and decide nothing. Rationale: the section
//! "Evidence packages" of `docs/adr/0086-bilateral-contract-signing.md`.

mod coverage;
mod pair;
mod parties;
#[cfg(test)]
mod tests;
mod timing;

use super::current::{self, Subject};
use super::revocation::{self, TsaList};
use super::token::{self, Candidates, TokenCheck};
use super::{in_entry, token_interval, Checked, Components, Evidence};
use crate::certificate;
use crate::contract_statements::decode_request;
use crate::crl::Crl;
use crate::manifest::contract::{
    self as manifest, DOCUMENT_ENTRY, DOCUMENT_TOKEN_ENTRY, REQUEST_ENTRY, ROOT_ENTRY,
    STATEMENT_TOKEN_ENTRY, STATEMENT_TSA_CHAIN_ENTRY, TSA_CHAIN_ENTRY,
};
use crate::receipt::contract as receipt;
use crate::report::{Finding, Instant, InstantSource, Profile, TrustNote};
use parties::{Expected, Instants, Party};
use timing::{Stamp, Token, Window};

pub(super) fn check(evidence: &Evidence<'_>) -> Checked {
    let full = evidence.layout.profile == Profile::BilateralContractV1;
    let policy = evidence.policy;
    let pdf_sha256 = certificate::sha256(evidence.entry(DOCUMENT_ENTRY));
    let request_bytes = evidence.entry(REQUEST_ENTRY);
    let root = super::pem_certificate(ROOT_ENTRY, evidence.entry(ROOT_ENTRY));
    let request = decode_request(request_bytes).map_err(|finding| in_entry(REQUEST_ENTRY, finding));
    let parties: Vec<Party<'_>> = (0..if full { 2 } else { 1 })
        .map(|index| Party::load(evidence, index))
        .collect();
    let client = &parties[0];
    let firm = parties.get(1);

    let stamp_over = |token: &str, covered: &[u8], chain: &str| {
        token::check(
            evidence.entry(token),
            &certificate::sha256(covered),
            &Candidates {
                chain_pem: evidence.archive.get(chain),
                issuer: root.as_ref().ok(),
            },
            policy,
        )
    };
    let t1_bytes = evidence.entry(STATEMENT_TOKEN_ENTRY);
    let mut t1 = stamp_over(
        STATEMENT_TOKEN_ENTRY,
        client.signature,
        STATEMENT_TSA_CHAIN_ENTRY,
    );
    let manifest_bytes = evidence.entry(manifest::ENTRY);
    let mut td = full.then(|| {
        stamp_over(
            DOCUMENT_TOKEN_ENTRY,
            evidence.entry(DOCUMENT_ENTRY),
            TSA_CHAIN_ENTRY,
        )
    });
    let mut tf = full.then(|| stamp_over(manifest::TOKEN_ENTRY, manifest_bytes, TSA_CHAIN_ENTRY));
    let parsed = full.then(|| {
        manifest::parse(manifest_bytes).map_err(|finding| in_entry(manifest::ENTRY, finding))
    });
    let facts = |check: &Option<TokenCheck>| check.as_ref().and_then(|check| check.facts.clone());
    let (t1_facts, td_facts, tf_facts) = (t1.facts.clone(), facts(&td), facts(&tf));
    let t1_at = t1_facts.as_ref().map(token_interval);
    let tf_at = tf_facts.as_ref().map(token_interval);
    let registered = |index: usize| {
        let manifest = parsed.as_ref()?.as_ref().ok()?;
        let at = manifest.signers[index].binding_registered_at;
        Some((at, at))
    };
    let instants: Vec<Instants> = parties
        .iter()
        .map(|party| match full {
            true => Instants::full(
                party.index,
                registered(party.index),
                [t1_at, tf_at][party.index],
            ),
            false => Instants::partial(t1_at),
        })
        .collect();

    let mut document_signature: Vec<Finding> =
        parties.iter().map(Party::signature_finding).collect();
    document_signature.extend(prefixed(STATEMENT_TOKEN_ENTRY, &t1.findings));

    let expected = Expected {
        request: request.as_ref().ok(),
        request_sha256: certificate::sha256(request_bytes),
        pdf_sha256,
        root_der: root.as_ref().ok().map(|root| root.der.as_slice()),
    };
    let mut binding_consent: Vec<Finding> = request.as_ref().err().cloned().into_iter().collect();
    for party in &parties {
        binding_consent.extend(party.consent(&expected));
    }

    let mut certificates = Vec::new();
    let mut revocation_findings = Vec::new();
    for (party, at) in parties.iter().zip(&instants) {
        certificates.extend(party.chain(&root, at, policy));
        revocation_findings.extend(party.revocation(root.as_ref().ok(), at));
    }
    certificates.extend(prefixed(STATEMENT_TOKEN_ENTRY, &t1.chain));
    revocation_findings.push(revocation::tsa_status(
        "T1 time-stamping certificate",
        t1.tsa.as_ref(),
        t1.facts.as_ref(),
        &designated(client),
    ));

    let mut context = Vec::new();
    let mut manifest_findings = None;
    let mut document_token = None;
    if let (Some(firm), Some(td), Some(tf), Some(parsed)) = (firm, &td, &tf, &parsed) {
        binding_consent.extend(pair::distinct(client, firm));
        for (entry, check) in [(DOCUMENT_TOKEN_ENTRY, td), (manifest::TOKEN_ENTRY, tf)] {
            certificates.extend(prefixed(entry, &check.chain));
        }
        let firm_lists = designated(firm);
        for (role, check) in [
            ("Td time-stamping certificate", td),
            ("Tf time-stamping certificate", tf),
        ] {
            revocation_findings.push(revocation::tsa_status(
                role,
                check.tsa.as_ref(),
                check.facts.as_ref(),
                &firm_lists,
            ));
        }
        revocation_findings.extend(pair::counterparty(client, firm, root.as_ref().ok(), tf_at));
        context.extend(pair::chain_link(firm, client.signature, t1_bytes));
        let mut findings = match parsed {
            Ok(manifest) => {
                for (index, label) in ["signer-1", "signer-2"].into_iter().enumerate() {
                    let signing = [&t1_facts, &tf_facts][index].as_ref().map(Stamp::of);
                    let at = manifest.signers[index].binding_registered_at;
                    context.extend(timing::registration(label, at, signing));
                }
                coverage::check(&coverage::Input {
                    evidence,
                    manifest,
                    request: request.as_ref().ok(),
                    request_sha256: expected.request_sha256,
                    pdf_sha256,
                    statements: [client.statement.as_ref().ok(), firm.statement.as_ref().ok()],
                })
            }
            Err(finding) => vec![finding.clone()],
        };
        findings.extend(prefixed(manifest::TOKEN_ENTRY, &tf.findings));
        manifest_findings = Some(findings);
        document_token = Some(td.findings.clone());
    }

    let stamp = |label, facts: &Option<crate::timestamp::TokenFacts>, bounded| Token {
        label,
        stamp: facts.as_ref().map(Stamp::of),
        bounded,
    };
    let mut tokens = vec![stamp("T1", &t1_facts, true)];
    if full {
        tokens.push(stamp("Td", &td_facts, false));
        tokens.push(stamp("Tf", &tf_facts, true));
    }
    let window = request.as_ref().ok().map(|request| Window {
        created_at: request.created_at,
        expires_at: request.expires_at,
    });
    context.extend(timing::check(window, &tokens));
    let mut instants_list = Vec::new();
    for (label, facts) in [
        ("T1 genTime", &t1_facts),
        ("Td genTime", &td_facts),
        ("Tf genTime", &tf_facts),
    ] {
        if let Some(facts) = facts {
            instants_list.push(token_instant(label, facts.gen_time));
        }
    }
    if let Some(Ok(manifest)) = &parsed {
        for (label, signer) in [
            "signer-1 binding_registered_at",
            "signer-2 binding_registered_at",
        ]
        .into_iter()
        .zip(&manifest.signers)
        {
            instants_list.push(record_instant(label, signer.binding_registered_at));
        }
    }
    if let Ok(request) = &request {
        instants_list.push(record_instant("request created_at", request.created_at));
        instants_list.push(record_instant("request expires_at", request.expires_at));
    }
    let (records, record_instants) = records(evidence, full);
    context.extend(records);
    instants_list.extend(record_instants);
    if !full {
        context.push(Finding::indeterminate(
            "contract.partial",
            "the package holds only the position 1 signature and does not establish a contract",
        ));
    }

    let mut subjects = Vec::new();
    let root_ok = root.as_ref().ok();
    for party in &parties {
        if let (Ok(certificate), Some(root)) = (&party.certificate, root_ok) {
            subjects.push(Subject {
                role: ["Client", "firm signer"][party.index],
                certificate,
                issuer: Some(root),
            });
        }
    }
    if let Some(root) = root_ok {
        subjects.push(Subject {
            role: "contract root",
            certificate: root,
            issuer: None,
        });
    }
    for check in std::iter::once(&t1).chain(td.iter()).chain(tf.iter()) {
        current::tsa_subjects(check.tsa.as_ref(), &mut subjects);
    }
    let package_crls: Vec<&Crl> = parties
        .iter()
        .flat_map(|party| [&party.crl, &party.binding_crl])
        .filter_map(|list| list.as_ref().ok())
        .collect();
    let current = current::status(&subjects, &package_crls, policy);

    let mut trust: Vec<TrustNote> = Vec::new();
    for notes in [
        std::mem::take(&mut t1.notes),
        td.as_mut()
            .map(|check| std::mem::take(&mut check.notes))
            .unwrap_or_default(),
        tf.as_mut()
            .map(|check| std::mem::take(&mut check.notes))
            .unwrap_or_default(),
    ] {
        for note in notes {
            if !trust.contains(&note) {
                trust.push(note);
            }
        }
    }

    Checked {
        components: Components {
            document_token,
            document_signature,
            binding_consent: Some(binding_consent),
            certificates,
            revocation: revocation_findings,
            manifest: manifest_findings,
            context,
        }
        .into_vec(),
        instants: instants_list,
        trust,
        current,
    }
}

/// The party's signature CRL as the list that decides for a token whose
/// time-stamping certificate it judges.
fn designated<'a>(party: &'a Party<'_>) -> Vec<TsaList<'a>> {
    party
        .crl
        .as_ref()
        .ok()
        .map(|crl| TsaList::Designated(party.entries.crl, crl))
        .into_iter()
        .collect()
}

/// `findings` with the entry they concern as the detail prefix.
fn prefixed(entry: &str, findings: &[Finding]) -> Vec<Finding> {
    findings
        .iter()
        .map(|finding| in_entry(entry, finding.clone()))
        .collect()
}

fn token_instant(label: &str, unix: i64) -> Instant {
    Instant {
        label: label.to_owned(),
        unix,
        source: InstantSource::Token,
    }
}

fn record_instant(label: &str, unix: i64) -> Instant {
    Instant {
        label: label.to_owned(),
        unix,
        source: InstantSource::ApplicationRecord,
    }
}

/// The schema of the receipt of the profile, and its instants as
/// application records; its values decide nothing.
fn records(evidence: &Evidence<'_>, full: bool) -> (Vec<Finding>, Vec<Instant>) {
    let mut instants = Vec::new();
    let failure = if full {
        match receipt::parse_receipt(evidence.entry(receipt::ENTRY)) {
            Ok(record) => {
                for (label, unix) in [
                    ("receipt client_accepted_at", record.client_accepted_at),
                    ("receipt firm_accepted_at", record.firm_accepted_at),
                    ("receipt completed_at", record.completed_at),
                ] {
                    instants.push(record_instant(label, unix));
                }
                None
            }
            Err(finding) => Some(in_entry(receipt::ENTRY, finding)),
        }
    } else {
        match receipt::parse_partial(evidence.entry(receipt::PARTIAL_ENTRY)) {
            Ok(record) => {
                for (label, unix) in [
                    (
                        "acknowledgement binding_registered_at",
                        record.binding_registered_at,
                    ),
                    ("acknowledgement accepted_at", record.accepted_at),
                ] {
                    instants.push(record_instant(label, unix));
                }
                None
            }
            Err(finding) => Some(in_entry(receipt::PARTIAL_ENTRY, finding)),
        }
    };
    (failure.into_iter().collect(), instants)
}

/// The names of the fields that do not hold, joined for a detail.
pub(super) fn differing(fields: &[(&str, bool)]) -> String {
    fields
        .iter()
        .filter(|(_, holds)| !holds)
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}
