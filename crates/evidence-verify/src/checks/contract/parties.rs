//! The material of each party and the checks that read one party, or both.
//!
//! Each party statement must agree with the request and the package, its
//! signature must verify under the party certificate, and the party's
//! `DOCBIND1` binding must describe the same binding as the statement. The
//! certificate chains to the pinned individual signing root through
//! `parte-N-vinculo-raiz.der` at the binding registration and through
//! `ca.pem` at the party's own token, and its revocation is read from
//! `parte-N-vinculo-crl.der` and `parte-N-crl.pem` at those instants.
//! Rationale: the section "Evidence packages" of
//! `docs/adr/0086-bilateral-contract-signing.md`.

use super::super::revocation::{self, RoleRevocation};
use super::super::trust::{self, Anchor, Chain};
use super::super::{der_certificate, der_crl, in_entry, pem_certificate, pem_crl};
use super::super::{signature_over, Evidence};
use super::differing;
use crate::certificate::{self, Certificate, Role};
use crate::contract_statements::{self, ContractRequest, PartyStatement};
use crate::crl::Crl;
use crate::manifest::contract::{PartyEntries, PARTY_ENTRIES, ROOT_ENTRY};
use crate::policy::{Policy, Purpose};
use crate::report::{Finding, Outcome};
use crate::statements::{self, BindingStatement};

/// The token instant each position is judged at.
pub(super) const SIGNING_INSTANT: [&str; 2] = ["T1 genTime", "Tf genTime"];
const SIGNATURE_ROLE: [&str; 2] = ["Client at T1 genTime", "firm signer at Tf genTime"];
const REGISTRATION_INSTANT: [&str; 2] = [
    "signer-1 binding_registered_at",
    "signer-2 binding_registered_at",
];
const BINDING_ROLE: [&str; 2] = [
    "Client binding at binding_registered_at",
    "firm signer binding at binding_registered_at",
];

/// The instants one party is judged at, with their report labels.
pub(super) struct Instants {
    /// What the binding instant is, as an anchor instant and as a role.
    pub registration_label: &'static str,
    pub registration_role: &'static str,
    pub registration: Option<(i64, i64)>,
    pub signing: Option<(i64, i64)>,
}

impl Instants {
    /// In the full package the manifest, covered by Tf, states when the
    /// binding was registered.
    pub fn full(
        index: usize,
        registration: Option<(i64, i64)>,
        signing: Option<(i64, i64)>,
    ) -> Self {
        Self {
            registration_label: REGISTRATION_INSTANT[index],
            registration_role: BINDING_ROLE[index],
            registration,
            signing,
        }
    }

    /// The partial package has no covered registration instant, so the
    /// Client binding is judged at T1.
    pub fn partial(t1: Option<(i64, i64)>) -> Self {
        Self {
            registration_label: "Client binding at T1 genTime",
            registration_role: "Client binding at T1 genTime",
            registration: t1,
            signing: t1,
        }
    }
}

/// What the statement of a party must agree with.
pub(super) struct Expected<'a> {
    pub request: Option<&'a ContractRequest>,
    pub request_sha256: [u8; 32],
    pub pdf_sha256: [u8; 32],
    /// `ca.pem` as DER, when it decoded.
    pub root_der: Option<&'a [u8]>,
}

/// The entries of the party at `index` (0 for position 1) and what they
/// decode to.
pub(super) struct Party<'a> {
    pub index: usize,
    pub entries: &'static PartyEntries,
    pub statement_bytes: &'a [u8],
    pub signature: &'a [u8],
    binding_bytes: &'a [u8],
    binding_signature: &'a [u8],
    binding_root_der: &'a [u8],
    pub certificate: Result<Certificate, Finding>,
    pub crl: Result<Crl, Finding>,
    binding_root: Result<Certificate, Finding>,
    pub binding_crl: Result<Crl, Finding>,
    pub statement: Result<PartyStatement, Finding>,
    binding: Result<BindingStatement, Finding>,
}

impl<'a> Party<'a> {
    pub fn load(evidence: &'a Evidence<'_>, index: usize) -> Self {
        let entries = &PARTY_ENTRIES[index];
        let entry = |name| evidence.entry(name);
        let statement_bytes = entry(entries.statement);
        let binding_bytes = entry(entries.binding_statement);
        let binding_root_der = entry(entries.binding_root);
        Self {
            index,
            entries,
            statement_bytes,
            signature: entry(entries.statement_signature),
            binding_bytes,
            binding_signature: entry(entries.binding_signature),
            binding_root_der,
            certificate: pem_certificate(entries.certificate, entry(entries.certificate)),
            crl: pem_crl(entries.crl, entry(entries.crl)),
            binding_root: der_certificate(entries.binding_root, binding_root_der),
            binding_crl: der_crl(entries.binding_crl, entry(entries.binding_crl)),
            statement: contract_statements::decode_party_statement(statement_bytes)
                .map_err(|finding| in_entry(entries.statement, finding)),
            binding: statements::decode_binding(binding_bytes)
                .map_err(|finding| in_entry(entries.binding_statement, finding)),
        }
    }

    /// The party's signature over its exact statement bytes.
    pub fn signature_finding(&self) -> Finding {
        in_entry(
            self.entries.statement_signature,
            signature_over(
                "signature.valid",
                "signature.invalid",
                self.certificate.as_ref().ok(),
                self.statement_bytes,
                self.signature,
            ),
        )
    }

    /// The statement against the request and the package, and the binding
    /// against the statement.
    pub fn consent(&self, expected: &Expected<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        match &self.statement {
            Ok(statement) => findings.extend(self.statement_matches(statement, expected)),
            Err(finding) => findings.push(finding.clone()),
        }
        findings.push(in_entry(
            self.entries.binding_signature,
            signature_over(
                "binding.signature_valid",
                "binding.signature_invalid",
                self.certificate.as_ref().ok(),
                self.binding_bytes,
                self.binding_signature,
            ),
        ));
        match &self.binding {
            Ok(binding) => findings.extend(self.binding_matches(binding)),
            Err(finding) => findings.push(finding.clone()),
        }
        let names_other_root = expected.root_der.is_some_and(|root| {
            let digest = certificate::sha256(root);
            self.binding_root_der != root
                || self
                    .statement
                    .as_ref()
                    .is_ok_and(|statement| statement.root_sha256 != digest)
        });
        if names_other_root {
            findings.push(Finding::failed(
                "contract.root_mismatch",
                format!(
                    "{} or {} does not name ca.pem",
                    self.entries.statement, self.entries.binding_root
                ),
            ));
        }
        findings
    }

    fn statement_matches(
        &self,
        statement: &PartyStatement,
        expected: &Expected<'_>,
    ) -> Vec<Finding> {
        let mut fields = vec![
            (
                "position",
                usize::from(statement.position) == self.index + 1,
            ),
            (
                "request_sha256",
                statement.request_sha256 == expected.request_sha256,
            ),
            (
                "document_sha256",
                statement.document_sha256 == expected.pdf_sha256,
            ),
        ];
        let mut party = Vec::new();
        if let Some(request) = expected.request {
            fields.extend([
                ("deployment", statement.deployment == request.deployment),
                ("request", statement.request == request.request),
                (
                    "request document_sha256",
                    request.document_sha256 == expected.pdf_sha256,
                ),
            ]);
            let (account, leaf) = match self.index {
                0 => (request.client_account, request.client_certificate_sha256),
                _ => (request.firm_account, request.firm_certificate_sha256),
            };
            party.push(("account", statement.account == account));
            party.push(("certificate_sha256", statement.certificate_sha256 == leaf));
        }
        if let Ok(certificate) = &self.certificate {
            let leaf = certificate::sha256(&certificate.der);
            party.push(("certificate entry", statement.certificate_sha256 == leaf));
        }
        let mut findings = Vec::new();
        for (code, fields) in [
            ("contract.statement_mismatch", fields),
            ("contract.party_mismatch", party),
        ] {
            let differing = differing(&fields);
            if !differing.is_empty() {
                findings.push(Finding::failed(
                    code,
                    format!("{} differs in {differing}", self.entries.statement),
                ));
            }
        }
        findings
    }

    fn binding_matches(&self, binding: &BindingStatement) -> Vec<Finding> {
        let mut findings = Vec::new();
        if binding.root_sha256 != certificate::sha256(self.binding_root_der) {
            findings.push(Finding::failed(
                "binding.root_mismatch",
                format!(
                    "{} does not name {}",
                    self.entries.binding_statement, self.entries.binding_root
                ),
            ));
        }
        if let Ok(certificate) = &self.certificate {
            if binding.certificate_sha256 != certificate::sha256(&certificate.der) {
                findings.push(Finding::failed(
                    "binding.certificate_mismatch",
                    format!(
                        "{} does not name {}",
                        self.entries.binding_statement, self.entries.certificate
                    ),
                ));
            }
        }
        let Ok(statement) = &self.statement else {
            return findings;
        };
        let fields = [
            ("deployment", binding.deployment == statement.deployment),
            ("account", binding.account == statement.account),
            ("binding", binding.binding == statement.binding),
            ("root_sha256", binding.root_sha256 == statement.root_sha256),
            (
                "certificate_sha256",
                binding.certificate_sha256 == statement.certificate_sha256,
            ),
            (
                "account_revision",
                binding.account_revision <= statement.account_revision,
            ),
            (
                "authentication_generation",
                binding.authentication_generation <= statement.authentication_generation,
            ),
            (
                "trust_revision",
                binding.trust_revision <= statement.trust_revision,
            ),
        ];
        let differing = differing(&fields);
        findings.push(if differing.is_empty() {
            Finding::new(
                Outcome::Checked,
                "statements.same_binding",
                format!(
                    "{} and {}",
                    self.entries.binding_statement, self.entries.statement
                ),
            )
        } else {
            Finding::failed(
                "statements.binding_mismatch",
                format!("{}: {differing}", self.entries.statement),
            )
        });
        findings
    }

    /// The party certificate, its anchors and their validity.
    pub fn chain(
        &self,
        root: &Result<Certificate, Finding>,
        at: &Instants,
        policy: &Policy,
    ) -> Vec<Finding> {
        let anchors = [
            Anchor {
                entry: self.entries.binding_root,
                root: &self.binding_root,
                instant: at.registration_label,
                at: at.registration,
            },
            Anchor {
                entry: ROOT_ENTRY,
                root,
                instant: SIGNING_INSTANT[self.index],
                at: at.signing,
            },
        ];
        trust::check_chain(
            &Chain {
                entry: self.entries.certificate,
                signer: &self.certificate,
                role: Role::IndividualSigner,
                purpose: Purpose::IndividualSigning,
                anchors: &anchors,
            },
            policy,
        )
    }

    /// The binding CRL at the registration and the signature CRL at the
    /// party's token, after the decoding failures of both lists.
    pub fn revocation(&self, root: Option<&Certificate>, at: &Instants) -> Vec<Finding> {
        let mut findings: Vec<Finding> = [&self.binding_crl, &self.crl]
            .iter()
            .filter_map(|list| list.as_ref().err().cloned())
            .collect();
        for check in [
            RoleRevocation {
                role: at.registration_role,
                subject: self.certificate.as_ref().ok(),
                issuer: self.binding_root.as_ref().ok(),
                crl: self.binding_crl.as_ref().ok(),
                at: at.registration,
            },
            RoleRevocation {
                role: SIGNATURE_ROLE[self.index],
                subject: self.certificate.as_ref().ok(),
                issuer: root,
                crl: self.crl.as_ref().ok(),
                at: at.signing,
            },
        ] {
            findings.extend(revocation::role_status(&check));
        }
        findings
    }
}
