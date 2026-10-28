//! Server seal v1: the document signed by the server seal signer and
//! time-stamped, with the signer checked at the document token's `genTime`.
//!
//! `ca.pem` is the signer's root and, without `tsa-chain.pem`, also the
//! documented token anchor; `crl.pem` is its CRL. Neither is trusted
//! because it is present. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use super::current::{self, Subject};
use super::revocation::{self, RoleRevocation};
use super::token::{self, Candidates};
use super::trust::{self, Anchor, Chain};
use super::{Checked, Components, Evidence};
use crate::certificate::{self, Role};
use crate::policy::Purpose;
use crate::report::{Finding, Instant, InstantSource, Outcome};

pub(super) fn check(evidence: &Evidence<'_>) -> Checked {
    let layout = evidence.layout;
    let policy = evidence.policy;
    let document = evidence.entry(&layout.document);
    let signature = evidence.entry(&layout.signature());
    let signer = super::pem_certificate("certificado.pem", evidence.entry("certificado.pem"));
    let root = super::pem_certificate("ca.pem", evidence.entry("ca.pem"));
    let token = token::check(
        evidence.entry(&layout.token()),
        &certificate::sha256(document),
        &Candidates {
            chain_pem: evidence.archive.get("tsa-chain.pem"),
            issuer: root.as_ref().ok(),
        },
        policy,
    );
    let at = token.facts.as_ref().map(super::token_interval);

    let document_signature = vec![super::signature_over(
        "signature.valid",
        "signature.invalid",
        signer.as_ref().ok(),
        document,
        signature,
    )];

    let anchors = [Anchor {
        entry: "ca.pem",
        root: &root,
        instant: "document token genTime",
        at,
    }];
    let certificates = trust::check_chain(
        &Chain {
            entry: "certificado.pem",
            signer: &signer,
            role: Role::ServerSealSigner,
            purpose: Purpose::ServerSeal,
            anchors: &anchors,
        },
        policy,
    );

    let crl = super::pem_crl("crl.pem", evidence.entry("crl.pem"));
    let mut revocation_findings: Vec<Finding> = crl.as_ref().err().cloned().into_iter().collect();
    revocation_findings.extend(revocation::role_status(&RoleRevocation {
        role: "server seal signer",
        subject: signer.as_ref().ok(),
        issuer: root.as_ref().ok(),
        crl: crl.as_ref().ok(),
        at,
    }));
    revocation_findings.push(revocation::tsa_status(
        token.tsa.as_ref(),
        token.facts.as_ref(),
        crl.as_ref().ok(),
    ));

    let mut instants = Vec::new();
    let context = match &token.facts {
        Some(facts) => {
            instants.push(Instant {
                label: "document token genTime".to_owned(),
                unix: facts.gen_time,
                source: InstantSource::Token,
            });
            vec![Finding::new(
                Outcome::Checked,
                "context.token_time",
                format!(
                    "genTime {} accuracy {} s",
                    facts.gen_time, facts.accuracy_seconds
                ),
            )]
        }
        None => vec![Finding::indeterminate(
            "context.token_time_unknown",
            "the document token did not yield a verified genTime",
        )],
    };

    let mut subjects = Vec::new();
    if let (Ok(signer), Ok(root)) = (&signer, &root) {
        subjects.push(Subject {
            role: "server seal signer",
            certificate: signer,
            issuer: Some(root),
        });
    }
    if let Ok(root) = &root {
        subjects.push(Subject {
            role: "server seal root",
            certificate: root,
            issuer: None,
        });
    }
    current::tsa_subjects(token.tsa.as_ref(), &mut subjects);
    let package_crls: Vec<_> = crl.as_ref().ok().into_iter().collect();
    let current = current::status(&subjects, &package_crls, policy);

    Checked {
        components: Components {
            document_token: token.findings,
            document_signature,
            binding_consent: None,
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
