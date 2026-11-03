//! The exact contracted version, read through the request by relation, with
//! the same integrity checks, size limit and incident recording as the
//! document content route, audited as `contract.content_read`.

use domain::document_signing::Uuid;

use super::{ContractService, Error};
use crate::{
    contracts::{
        access::Relation, model::ContractSnapshot, port::ContractRead, state::request_status,
    },
    document_content::{DocumentContent, MAX_DOCUMENT_CONTENT_BYTES},
    document_integrity::{DocumentIntegrityObservation, DocumentIntegrityObservationId},
    documents::DocumentRecord,
    identity::Principal,
    ApplicationError,
};
use domain::crypto::DocumentVersionRef;
use zeroize::Zeroizing;

/// The exact PDF of a request.
pub type ContractContent = DocumentContent;

impl ContractService {
    pub fn content(&self, token: &str, request: Uuid) -> Result<ContractContent, ApplicationError> {
        let principal = self.principal(token)?;
        let snapshot = self.by_request(&principal, request)?;
        let record = snapshot.request(request).ok_or(Error::Inconsistent)?;
        let status = request_status(&snapshot, record, self.now().unix_timestamp());
        Relation::of(&principal, &snapshot, Some(record))
            .read_content(status, record.signature(1).is_some())?;
        let (document, bytes) = self.plaintext(token, &principal, &snapshot, request)?;
        self.reauthenticate(token, &principal)?;
        self.ports
            .store
            .record_read(&principal, request, ContractRead::Content, self.now())?;
        let fields = record.request.fields();
        Ok(DocumentContent {
            case_id: fields.case,
            reference: DocumentVersionRef {
                id: document.id,
                version: document.version,
            },
            file_name: document.name,
            digest: document.digest,
            bytes,
        })
    }

    /// Decrypts and checks the exact version a request names; a rejected
    /// content records its integrity incident before failing.
    pub(super) fn plaintext(
        &self,
        token: &str,
        principal: &Principal,
        snapshot: &ContractSnapshot,
        request: Uuid,
    ) -> Result<(DocumentRecord, Zeroizing<Vec<u8>>), ApplicationError> {
        let fields = *snapshot
            .request(request)
            .ok_or(Error::Inconsistent)?
            .request
            .fields();
        let document = self.ports.store.request_document(principal, request)?;
        if document.id != fields.document.id
            || document.version != fields.document.version
            || document.digest != fields.content_digest
        {
            return Err(Error::Inconsistent.into());
        }
        let bytes = match self.ports.processor.content_plaintext(&document) {
            Ok(bytes) => bytes,
            Err(ApplicationError::DocumentContentValidationFailed(failure)) => {
                let authority = self.reauthenticate(token, principal);
                let observation = DocumentIntegrityObservation {
                    observation_id: DocumentIntegrityObservationId::new(),
                    requester: principal.id,
                    case_id: fields.case,
                    record: document,
                    failure,
                    detected_at: self.now(),
                };
                let persisted = self.ports.incidents.record_rejection(&observation);
                authority?;
                persisted?;
                return Err(ApplicationError::DocumentContentValidationFailed(failure));
            }
            Err(error) => return Err(error),
        };
        if bytes.len() > MAX_DOCUMENT_CONTENT_BYTES {
            return Err(ApplicationError::DocumentContentTooLarge);
        }
        Ok((document, bytes))
    }
}
