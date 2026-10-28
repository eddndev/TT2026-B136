//! Individual signing v1: the document signed by the holder and
//! time-stamped, and the binding and consent statements signed by the same
//! holder.
//!
//! The binding is checked at `binding_registered_at` with
//! `binding-root.der` and `binding-crl.der`; the document signature and the
//! consent at `accepted_at` with `ca.pem` and `crl.pem`. Both instants are
//! application records that no time-stamp covers in this profile.
//! Rationale: `docs/adr/0085-in-browser-evidence-validation.md`.

mod consistency;
mod statements;

use super::current::{self, Subject};
use super::revocation::{self, RoleRevocation};
use super::token::{self, Candidates};
use super::trust::{self, Anchor, Chain};
use super::{Checked, Components, Evidence};
use crate::certificate::{self, Role};
use crate::individual_json;
use crate::policy::Purpose;
use crate::report::{Finding, Instant, InstantSource};

pub(super) fn check(evidence: &Evidence<'_>) -> Checked {
    let layout = evidence.layout;
    let policy = evidence.policy;
    let document = evidence.entry(&layout.document);
    let signature = evidence.entry(&layout.signature());
    let holder = super::pem_certificate("certificado.pem", evidence.entry("certificado.pem"));
    let root = super::pem_certificate("ca.pem", evidence.entry("ca.pem"));
    let binding_root_der = evidence.entry("binding-root.der");
    let binding_root = super::der_certificate("binding-root.der", binding_root_der);
    let record = individual_json::parse(evidence.entry("individual-signing.json"));
    let token = token::check(
        evidence.entry(&layout.token()),
        &certificate::sha256(document),
        &Candidates {
            chain_pem: evidence.archive.get("tsa-chain.pem"),
            issuer: root.as_ref().ok(),
        },
        policy,
    );
    let point = |instant: i64| (instant, instant);
    let registered = record
        .as_ref()
        .ok()
        .map(|record| point(record.binding_registered_at));
    let accepted = record.as_ref().ok().map(|record| point(record.accepted_at));

    let document_signature = vec![super::signature_over(
        "signature.valid",
        "signature.invalid",
        holder.as_ref().ok(),
        document,
        signature,
    )];

    let (binding_consent, decoded) = statements::check(&statements::Input {
        evidence,
        record: &record,
        holder: holder.as_ref().ok(),
        root: root.as_ref().ok(),
        document,
        signature,
    });

    let anchors = [
        Anchor {
            entry: "binding-root.der",
            root: &binding_root,
            instant: "binding_registered_at",
            at: registered,
        },
        Anchor {
            entry: "ca.pem",
            root: &root,
            instant: "accepted_at",
            at: accepted,
        },
    ];
    let mut certificates = trust::check_chain(
        &Chain {
            entry: "certificado.pem",
            signer: &holder,
            role: Role::IndividualSigner,
            purpose: Purpose::IndividualSigning,
            anchors: &anchors,
        },
        policy,
    );
    certificates.extend(token.chain);

    let binding_crl = super::der_crl("binding-crl.der", evidence.entry("binding-crl.der"));
    let crl = super::pem_crl("crl.pem", evidence.entry("crl.pem"));
    let mut revocation_findings: Vec<Finding> = [&binding_crl, &crl]
        .iter()
        .filter_map(|list| list.as_ref().err().cloned())
        .collect();
    for check in [
        RoleRevocation {
            role: "holder at binding_registered_at",
            subject: holder.as_ref().ok(),
            issuer: binding_root.as_ref().ok(),
            crl: binding_crl.as_ref().ok(),
            at: registered,
        },
        RoleRevocation {
            role: "holder at accepted_at",
            subject: holder.as_ref().ok(),
            issuer: root.as_ref().ok(),
            crl: crl.as_ref().ok(),
            at: accepted,
        },
    ] {
        revocation_findings.extend(revocation::role_status(&check));
    }
    revocation_findings.push(revocation::tsa_status(
        token.tsa.as_ref(),
        token.facts.as_ref(),
        crl.as_ref().ok(),
    ));

    let context = consistency::check(&consistency::Input {
        record: record.as_ref().ok(),
        token: token.facts.as_ref(),
        statements: decoded.as_ref(),
        root: root.as_ref().ok(),
        binding_root_der,
        crl: crl.as_ref().ok(),
        binding_crl: binding_crl.as_ref().ok(),
        binding_crl_der: evidence.entry("binding-crl.der"),
    });

    let mut instants = Vec::new();
    if let Some(facts) = &token.facts {
        instants.push(Instant {
            label: "document token genTime".to_owned(),
            unix: facts.gen_time,
            source: InstantSource::Token,
        });
    }
    if let Ok(record) = &record {
        for (label, unix) in [
            ("binding_registered_at", record.binding_registered_at),
            ("accepted_at", record.accepted_at),
        ] {
            instants.push(Instant {
                label: label.to_owned(),
                unix,
                source: InstantSource::ApplicationRecord,
            });
        }
    }

    let mut subjects = Vec::new();
    if let (Ok(holder), Ok(root)) = (&holder, &root) {
        subjects.push(Subject {
            role: "holder",
            certificate: holder,
            issuer: Some(root),
        });
    }
    if let Ok(root) = &root {
        subjects.push(Subject {
            role: "holder root",
            certificate: root,
            issuer: None,
        });
    }
    current::tsa_subjects(token.tsa.as_ref(), &mut subjects);
    let package_crls: Vec<_> = [&crl, &binding_crl]
        .into_iter()
        .filter_map(|list| list.as_ref().ok())
        .collect();
    let current = current::status(&subjects, &package_crls, policy);

    Checked {
        components: Components {
            document_token: token.findings,
            document_signature,
            binding_consent: Some(binding_consent),
            certificates,
            revocation: revocation_findings,
            manifest: None,
            context,
        }
        .into_vec(),
        instants,
        trust: token.notes,
        current,
    }
}
