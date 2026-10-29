//! Individual signing v1: the document signed by the holder and
//! time-stamped, and the binding and consent statements signed by the same
//! holder.
//!
//! The binding is checked at `binding_registered_at` with
//! `binding-root.der` and `binding-crl.der`; the document signature and the
//! consent at `accepted_at` with `ca.pem` and `crl.pem`. Both instants are
//! application records that no time-stamp covers in this profile.
//! [`assemble`] also yields the parts that individual signing v2 extends
//! with its manifest. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

mod consistency;
pub(super) mod manifest_context;
mod statements;

use super::current::{self, Subject};
use super::revocation::{self, RoleRevocation, TsaList};
use super::token::{self, Candidates, TokenCheck};
use super::trust::{self, Anchor, Chain};
use super::{Checked, Components, Evidence};
use crate::certificate::{self, Certificate, Role};
use crate::crl::Crl;
use crate::individual_json::{self, IndividualSigning};
use crate::policy::Purpose;
use crate::report::{Component, Finding, Instant, InstantSource, Profile};
use crate::statements::{BindingStatement, ConsentStatement};

/// Everything individual signing v1 checks, before the revocation of the
/// time-stamping certificate and the current status, which depend on the
/// CRLs a profile offers for it.
pub(super) struct Parts {
    pub document_signature: Vec<Finding>,
    pub binding_consent: Vec<Finding>,
    pub certificates: Vec<Finding>,
    /// Revocation of the holder at both instants and CRL decoding failures.
    pub revocation: Vec<Finding>,
    pub context: Vec<Finding>,
    pub instants: Vec<Instant>,
    /// The document token; its findings form the document token component
    /// and its chain findings are already in `certificates`.
    pub token: TokenCheck,
    pub crl: Result<Crl, Finding>,
    pub binding_crl: Result<Crl, Finding>,
    pub record: Result<IndividualSigning, Finding>,
    pub statements: Option<(BindingStatement, ConsentStatement)>,
    pub holder: Result<Certificate, Finding>,
    pub root: Result<Certificate, Finding>,
}

impl Parts {
    /// The certificates of the current status: the holder, its root and the
    /// document token's time-stamping chain.
    pub fn subjects(&self) -> Vec<Subject<'_>> {
        let mut subjects = Vec::new();
        if let (Ok(holder), Ok(root)) = (&self.holder, &self.root) {
            subjects.push(Subject {
                role: "holder",
                certificate: holder,
                issuer: Some(root),
            });
        }
        if let Ok(root) = &self.root {
            subjects.push(Subject {
                role: "holder root",
                certificate: root,
                issuer: None,
            });
        }
        current::tsa_subjects(self.token.tsa.as_ref(), &mut subjects);
        subjects
    }

    /// The CRL entries that decoded, as fallbacks of the current status.
    pub fn package_crls(&self) -> Vec<&Crl> {
        [&self.crl, &self.binding_crl]
            .into_iter()
            .filter_map(|list| list.as_ref().ok())
            .collect()
    }

    /// The components of the report, with `manifest` for the manifest
    /// component or not applicable when it is `None`.
    pub fn into_components(self, manifest: Option<Vec<Finding>>) -> Vec<Component> {
        Components {
            document_token: self.token.findings,
            document_signature: self.document_signature,
            binding_consent: Some(self.binding_consent),
            certificates: self.certificates,
            revocation: self.revocation,
            manifest,
            context: self.context,
        }
        .into_vec()
    }
}

/// Individual signing v1: the parts, with `crl.pem` for the time-stamping
/// certificate when its issuer issued that list.
pub(super) fn check(evidence: &Evidence<'_>) -> Checked {
    let mut parts = assemble(evidence);
    let lists: Vec<TsaList<'_>> = parts
        .crl
        .as_ref()
        .ok()
        .map(|crl| TsaList::IfIssuer("crl.pem", crl))
        .into_iter()
        .collect();
    let tsa = revocation::tsa_status(
        "time-stamping certificate",
        parts.token.tsa.as_ref(),
        parts.token.facts.as_ref(),
        &lists,
    );
    let current = current::status(&parts.subjects(), &parts.package_crls(), evidence.policy);
    parts.revocation.push(tsa);
    let trust = std::mem::take(&mut parts.token.notes);
    Checked {
        instants: std::mem::take(&mut parts.instants),
        components: parts.into_components(None),
        trust,
        current,
    }
}

/// Check everything individual signing v1 and v2 share.
pub(super) fn assemble(evidence: &Evidence<'_>) -> Parts {
    let layout = evidence.layout;
    let policy = evidence.policy;
    let document = evidence.entry(&layout.document);
    let signature = evidence.entry(&layout.signature());
    let holder = super::pem_certificate("certificado.pem", evidence.entry("certificado.pem"));
    let root = super::pem_certificate("ca.pem", evidence.entry("ca.pem"));
    let binding_root_der = evidence.entry("binding-root.der");
    let binding_root = super::der_certificate("binding-root.der", binding_root_der);
    let record = individual_json::parse(evidence.entry("individual-signing.json"));
    let mut token = token::check(
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
    certificates.append(&mut token.chain);

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

    let context = consistency::check(&consistency::Input {
        record: record.as_ref().ok(),
        token: token.facts.as_ref(),
        statements: decoded.as_ref(),
        root: root.as_ref().ok(),
        binding_root_der,
        crl: crl.as_ref().ok(),
        binding_crl: binding_crl.as_ref().ok(),
        binding_crl_der: evidence.entry("binding-crl.der"),
        manifest: layout.profile == Profile::IndividualSigningV2,
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

    Parts {
        document_signature,
        binding_consent,
        certificates,
        revocation: revocation_findings,
        context,
        instants,
        token,
        crl,
        binding_crl,
        record,
        statements: decoded,
        holder,
        root,
    }
}
