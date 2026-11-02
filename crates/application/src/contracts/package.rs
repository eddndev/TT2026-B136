//! The exact entries of both contract package profiles, built from stored
//! bytes with the entry names of `evidence_verify`, so the manifest covers
//! exactly what the export writes. Certificates are PEM-encoded once, here,
//! for both the manifest and the archive. Rationale: the section "Evidence
//! packages" of `docs/adr/0086-bilateral-contract-signing.md`.

use domain::identity::Role;
use evidence_verify::manifest::contract::{
    encode, AccountRole, ContractManifest, ContractRole, ContractSigner, PartyEntries,
    DOCUMENT_ENTRY, DOCUMENT_TOKEN_ENTRY, PARTY_ENTRIES, REQUEST_ENTRY, ROOT_ENTRY,
    STATEMENT_TOKEN_ENTRY, STATEMENT_TSA_CHAIN_ENTRY, TSA_CHAIN_ENTRY,
};

use super::{
    error::ContractWorkflowError as Error,
    model::{RequestRecord, StatementToken},
};
use crate::{
    document_signing::{pem, SigningBindingReceipt},
    ApplicationError,
};

/// One party's signed statement and the material that judges it.
pub(crate) struct PartyMaterial<'a> {
    pub(crate) statement: Vec<u8>,
    pub(crate) signature: &'a [u8],
    /// CRL of the trust revision the statement names.
    pub(crate) crl_der: &'a [u8],
    pub(crate) binding: &'a SigningBindingReceipt,
    pub(crate) operation: domain::document_signing::Uuid,
}

/// Everything both profiles draw their entries from.
pub(crate) struct PackageSource<'a> {
    pub(crate) pdf: &'a [u8],
    pub(crate) request: &'a RequestRecord,
    pub(crate) root_der: &'a [u8],
    pub(crate) t1: &'a StatementToken,
    pub(crate) parties: Vec<PartyMaterial<'a>>,
}

/// One package entry with its manifest party and role.
pub(crate) struct Entry {
    pub(crate) name: &'static str,
    pub(crate) party: u8,
    pub(crate) role: ContractRole,
    pub(crate) bytes: Vec<u8>,
}

fn entry(name: &'static str, party: u8, role: ContractRole, bytes: Vec<u8>) -> Entry {
    Entry {
        name,
        party,
        role,
        bytes,
    }
}

fn party_entries(names: &PartyEntries, number: u8, party: &PartyMaterial<'_>) -> Vec<Entry> {
    let binding = party.binding;
    vec![
        entry(
            names.statement,
            number,
            ContractRole::Statement,
            party.statement.clone(),
        ),
        entry(
            names.statement_signature,
            number,
            ContractRole::StatementSignature,
            party.signature.to_vec(),
        ),
        entry(
            names.certificate,
            number,
            ContractRole::SignerCertificate,
            pem("CERTIFICATE", &binding.check.certificate.der),
        ),
        entry(
            names.crl,
            number,
            ContractRole::IssuerCrl,
            pem("X509 CRL", party.crl_der),
        ),
        entry(
            names.binding_statement,
            number,
            ContractRole::BindingStatement,
            binding.statement.canonical_bytes().to_vec(),
        ),
        entry(
            names.binding_signature,
            number,
            ContractRole::BindingSignature,
            binding.check.signature.as_bytes().to_vec(),
        ),
        entry(
            names.binding_root,
            number,
            ContractRole::BindingRoot,
            binding.trust.inspection.root_der.clone(),
        ),
        entry(
            names.binding_crl,
            number,
            ContractRole::BindingCrl,
            binding.trust.inspection.crl_der.clone(),
        ),
    ]
}

impl PackageSource<'_> {
    /// The entries shared by both profiles: the PDF, the request, the root,
    /// every position 1 entry and T1 with its chain.
    pub(crate) fn partial_entries(&self) -> Result<Vec<Entry>, ApplicationError> {
        let first = self.parties.first().ok_or(Error::Inconsistent)?;
        let mut entries = vec![
            entry(DOCUMENT_ENTRY, 0, ContractRole::Document, self.pdf.to_vec()),
            entry(
                REQUEST_ENTRY,
                0,
                ContractRole::Request,
                self.request.bytes().to_vec(),
            ),
            entry(
                ROOT_ENTRY,
                0,
                ContractRole::IssuerCertificate,
                pem("CERTIFICATE", self.root_der),
            ),
        ];
        entries.extend(party_entries(&PARTY_ENTRIES[0], 1, first));
        entries.push(entry(
            STATEMENT_TOKEN_ENTRY,
            1,
            ContractRole::StatementTimestamp,
            self.t1.token.clone(),
        ));
        entries.push(entry(
            STATEMENT_TSA_CHAIN_ENTRY,
            1,
            ContractRole::StatementTsaChain,
            self.t1.tsa_chain_pem.clone(),
        ));
        Ok(entries)
    }

    /// Every entry the manifest covers: the partial entries, Td, the
    /// shared time-stamping chain and every position 2 entry.
    pub(crate) fn covered_entries(
        &self,
        document_token: &[u8],
        tsa_chain_pem: &[u8],
    ) -> Result<Vec<Entry>, ApplicationError> {
        let second = self.parties.get(1).ok_or(Error::Inconsistent)?;
        let mut entries = self.partial_entries()?;
        entries.push(entry(
            DOCUMENT_TOKEN_ENTRY,
            0,
            ContractRole::DocumentTimestamp,
            document_token.to_vec(),
        ));
        entries.push(entry(
            TSA_CHAIN_ENTRY,
            0,
            ContractRole::TsaChain,
            tsa_chain_pem.to_vec(),
        ));
        entries.extend(party_entries(&PARTY_ENTRIES[1], 2, second));
        Ok(entries)
    }

    /// The canonical manifest over `entries`, refused unless the validator
    /// would admit it.
    pub(crate) fn manifest(
        &self,
        entries: &[Entry],
        firm_role: Role,
    ) -> Result<Vec<u8>, ApplicationError> {
        let fields = self.request.request.fields();
        let role = match firm_role {
            Role::Owner => AccountRole::Owner,
            Role::Litigator => AccountRole::Litigator,
            _ => return Err(Error::Inconsistent.into()),
        };
        let signer = |party: &PartyMaterial<'_>, account_role| ContractSigner {
            account_id: party.binding.statement.account().id().as_uuid().to_string(),
            account_role,
            binding_id: party.binding.statement.material().binding().to_string(),
            binding_registered_at: party.binding.registered_at.unix_timestamp(),
            operation_id: party.operation.to_string(),
        };
        let (first, second) = match self.parties.as_slice() {
            [first, second] => (first, second),
            _ => return Err(Error::Inconsistent.into()),
        };
        let manifest = ContractManifest {
            deployment_id: fields.deployment.to_string(),
            case_id: fields.case.as_uuid().to_string(),
            request_id: fields.request.to_string(),
            document_id: fields.document.id.as_uuid().to_string(),
            document_version: u64::from(fields.document.version.get()),
            document_sha256: *fields.content_digest.as_bytes(),
            request_sha256: evidence_verify::certificate::sha256(&self.request.bytes()),
            expires_at: fields.expires_at_seconds,
            artifacts: entries
                .iter()
                .map(|e| ContractManifest::artifact(e.party, e.role, &e.bytes))
                .collect(),
            signers: [signer(first, AccountRole::Client), signer(second, role)],
        };
        encode(&manifest).map_err(|_| Error::Inconsistent.into())
    }
}
