//! The partial and the full evidence packages, with exactly the entries of
//! the `BilateralContractPartialV1` and `BilateralContractV1` profiles plus
//! `INSTRUCCIONES.md`, audited as `contract.evidence_exported`. The full
//! package must reproduce every artifact its stored manifest covers.

use domain::crypto::ArchiveEntry;
use domain::document_signing::Uuid;
use evidence_verify::{
    manifest::contract::{self as manifest, ContractManifest},
    profile::INSTRUCTIONS,
    receipt::contract::{self as receipts, ContractReceipt, PartialReceipt},
};

use super::{ContractService, Error};
use crate::{
    contracts::{
        access::Relation,
        model::{PartySignature, RequestRecord},
        package::{Entry, PackageSource, PartyMaterial},
        port::ContractRead,
    },
    ApplicationError,
};

const TEMPLATE: &str = include_str!("../INSTRUCCIONES-CONTRATO.template.md");
const FULL_SECTION: &str = include_str!("../INSTRUCCIONES-COMPLETO.template.md");

/// Which package of a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractStage {
    Partial,
    Final,
}

impl ContractStage {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "partial" => Some(Self::Partial),
            "final" => Some(Self::Final),
            _ => None,
        }
    }
}

/// One built evidence archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractPackage {
    pub request: Uuid,
    pub stage: ContractStage,
    pub archive: Vec<u8>,
}

impl ContractService {
    pub fn export(
        &self,
        token: &str,
        request: Uuid,
        stage: ContractStage,
    ) -> Result<ContractPackage, ApplicationError> {
        let principal = self.principal(token)?;
        let snapshot = self.by_request(&principal, request)?;
        let record = snapshot.request(request).ok_or(Error::Inconsistent)?;
        let relation = Relation::of(&principal, &snapshot, Some(record));
        let first = record.signature(1);
        match stage {
            ContractStage::Partial => relation.partial_package(first.is_some())?,
            ContractStage::Final => relation.final_package()?,
        }
        let first = first.ok_or(Error::NotComplete)?;
        if stage == ContractStage::Final && record.completion.is_none() {
            return Err(Error::NotComplete.into());
        }
        let (_, pdf) = self.plaintext(token, &principal, &snapshot, request)?;
        let at = self.now();
        let mut entries = match stage {
            ContractStage::Partial => partial(record, first, &pdf, at.unix_timestamp())?,
            ContractStage::Final => full(record, first, &pdf)?,
        };
        entries.push(ArchiveEntry::new(
            INSTRUCTIONS,
            instructions(record, stage).into_bytes(),
        )?);
        let archive = self.ports.archiver.write_archive(&entries)?;
        self.reauthenticate(token, &principal)?;
        let read = match stage {
            ContractStage::Partial => ContractRead::PartialEvidence,
            ContractStage::Final => ContractRead::FinalEvidence,
        };
        self.ports
            .store
            .record_read(&principal, request, read, at)?;
        Ok(ContractPackage {
            request,
            stage,
            archive,
        })
    }
}

fn party<'a>(
    signature: &'a PartySignature,
    binding: &'a crate::document_signing::SigningBindingReceipt,
) -> PartyMaterial<'a> {
    PartyMaterial {
        statement: signature.statement.canonical_bytes().to_vec(),
        signature: &signature.signature,
        crl_der: &signature.crl_der,
        binding,
        operation: signature.statement.operation(),
    }
}

fn archive_entries(entries: Vec<Entry>) -> Result<Vec<ArchiveEntry>, ApplicationError> {
    entries
        .into_iter()
        .map(|entry| ArchiveEntry::new(entry.name, entry.bytes).map_err(Into::into))
        .collect()
}

fn partial(
    record: &RequestRecord,
    first: &PartySignature,
    pdf: &[u8],
    exported_at: i64,
) -> Result<Vec<ArchiveEntry>, ApplicationError> {
    let t1 = first.timestamp.as_ref().ok_or(Error::Inconsistent)?;
    let source = PackageSource {
        pdf,
        request: record,
        root_der: &first.root_der,
        t1,
        parties: vec![party(first, &record.client_binding)],
    };
    let mut entries = archive_entries(source.partial_entries()?)?;
    let acknowledgement = receipts::encode_partial(&PartialReceipt {
        request_id: record.id().to_string(),
        request_sha256: evidence_verify::certificate::sha256(&record.bytes()),
        binding_registered_at: record.client_binding.registered_at.unix_timestamp(),
        accepted_at: first.accepted_at.unix_timestamp(),
        ceremony_reference: record.ceremony_reference.clone(),
        exported_at,
    })
    .map_err(|_| Error::Inconsistent)?;
    entries.push(ArchiveEntry::new(receipts::PARTIAL_ENTRY, acknowledgement)?);
    Ok(entries)
}

fn full(
    record: &RequestRecord,
    first: &PartySignature,
    pdf: &[u8],
) -> Result<Vec<ArchiveEntry>, ApplicationError> {
    let completion = record.completion.as_ref().ok_or(Error::NotComplete)?;
    let second = record.signature(2).ok_or(Error::Inconsistent)?;
    let t1 = first.timestamp.as_ref().ok_or(Error::Inconsistent)?;
    let source = PackageSource {
        pdf,
        request: record,
        root_der: &second.root_der,
        t1,
        parties: vec![
            party(first, &record.client_binding),
            party(second, &record.firm_binding),
        ],
    };
    let covered = source.covered_entries(&completion.document_token, &completion.tsa_chain_pem)?;
    let stored = manifest::parse(&completion.manifest).map_err(|_| Error::Inconsistent)?;
    let mut expected: Vec<_> = covered
        .iter()
        .map(|e| ContractManifest::artifact(e.party, e.role, &e.bytes))
        .collect();
    expected.sort_by(|a, b| a.entry.as_bytes().cmp(b.entry.as_bytes()));
    if stored.artifacts != expected {
        return Err(Error::Inconsistent.into());
    }
    let mut entries = archive_entries(covered)?;
    let receipt = receipts::encode_receipt(&ContractReceipt {
        request_id: record.id().to_string(),
        request_sha256: evidence_verify::certificate::sha256(&record.bytes()),
        ceremony_reference: record.ceremony_reference.clone(),
        client_accepted_at: first.accepted_at.unix_timestamp(),
        firm_accepted_at: second.accepted_at.unix_timestamp(),
        completed_at: completion.completed_at.unix_timestamp(),
    })
    .map_err(|_| Error::Inconsistent)?;
    entries.push(ArchiveEntry::new(
        manifest::ENTRY,
        completion.manifest.clone(),
    )?);
    entries.push(ArchiveEntry::new(
        manifest::TOKEN_ENTRY,
        completion.manifest_token.clone(),
    )?);
    entries.push(ArchiveEntry::new(receipts::ENTRY, receipt)?);
    Ok(entries)
}

fn instructions(record: &RequestRecord, stage: ContractStage) -> String {
    let fields = record.request.fields();
    let (profile, section) = match stage {
        ContractStage::Partial => ("acuse parcial, no acredita contrato", ""),
        ContractStage::Final => ("contrato completo", FULL_SECTION),
    };
    TEMPLATE
        .replace("{{PERFIL}}", profile)
        .replace("{{SOLICITUD}}", &record.id().to_string())
        .replace(
            "{{SOLICITUD_DIGEST}}",
            &evidence_verify::report::hex(&evidence_verify::certificate::sha256(&record.bytes())),
        )
        .replace("{{PDF_DIGEST}}", &fields.content_digest.to_hex())
        .replace("{{VENCE}}", &fields.expires_at_seconds.to_string())
        .replace("{{COMPLETO}}\n", section)
}

#[cfg(test)]
#[path = "export_tests.rs"]
mod tests;
