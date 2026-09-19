//! Durable document integrity reporting and the Owner security inbox.

use application::document_integrity::{
    DocumentIntegrityIncident, DocumentIntegrityIncidentId, DocumentIntegrityObservation,
    DocumentIntegrityPage, DocumentIntegrityQuery, DocumentIntegrityReceipt,
    DocumentIntegrityStore,
};
use application::ApplicationError;
use domain::crypto::{DocumentHasher, DocumentVersionRef};
use domain::identity::{Role, UserId};
use postgres::{GenericClient, Transaction};
use time::OffsetDateTime;

use super::{integrity_codec as codec, storage::port_error, PostgresCaseDocumentStore};
use crate::{
    audit_postgres::{append_transaction, begin_audited},
    postgres_actor::active_actor,
    RingSha256Hasher,
};

impl DocumentIntegrityStore for PostgresCaseDocumentStore {
    fn record_rejection(
        &self,
        observation: &DocumentIntegrityObservation,
    ) -> Result<DocumentIntegrityReceipt, ApplicationError> {
        if !codec::valid_time(observation.detected_at) {
            return Err(ApplicationError::InvalidInput(
                "document observation requires a UTC instant".into(),
            ));
        }
        let snapshot = codec::snapshot(observation);
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        if let Some(row) = tx
            .query_opt(
                "SELECT * FROM document_integrity_incidents WHERE observation_id=$1",
                &[&observation.observation_id.as_uuid()],
            )
            .map_err(port_error)?
        {
            let prior = codec::decode(&row)?;
            if prior.case_id != observation.case_id
                || prior.reference.id != observation.record.id
                || prior.reference.version != observation.record.version
                || prior.requester != observation.requester
                || prior.failure != observation.failure
                || prior.detected_at != observation.detected_at
                || prior.expected_digest != observation.record.digest
                || prior.observed_snapshot_digest != snapshot
            {
                return Err(ApplicationError::DocumentIntegrityObservationConflict);
            }
            tx.commit().map_err(port_error)?;
            return Ok(receipt(&prior));
        }
        let at = OffsetDateTime::now_utc();
        if observation.detected_at > at {
            return Err(ApplicationError::InvalidInput(
                "document observation is in the future".into(),
            ));
        }
        // Reporting preserves the authorized observation even after the requester's access is revoked.
        let exists: bool = tx
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM documents WHERE id=$1 AND case_id=$2 AND version=$3)",
                &[
                    &observation.record.id.as_uuid(),
                    &observation.case_id.as_uuid(),
                    &i64::from(observation.record.version.get()),
                ],
            )
            .map_err(port_error)?
            .get(0);
        if !exists {
            return Err(codec::inconsistent());
        }
        let incident = DocumentIntegrityIncident {
            id: DocumentIntegrityIncidentId::new(),
            observation_id: observation.observation_id,
            case_id: observation.case_id,
            reference: DocumentVersionRef {
                id: observation.record.id,
                version: observation.record.version,
            },
            requester: observation.requester,
            failure: observation.failure,
            detected_at: observation.detected_at,
            recorded_at: at,
            expected_digest: observation.record.digest,
            observed_snapshot_digest: snapshot,
        };
        insert(&mut tx, &incident)?;
        append_transaction(
            &mut tx,
            "system:document-integrity",
            "document.content_rejected",
            &format!(
                "case:{}:document:{}:version:{}:incident:{}",
                incident.case_id,
                incident.reference.id,
                incident.reference.version.get(),
                incident.id
            ),
            at,
        )?;
        tx.commit().map_err(port_error)?;
        Ok(receipt(&incident))
    }

    fn list(
        &self,
        actor: UserId,
        query: DocumentIntegrityQuery,
        at: OffsetDateTime,
    ) -> Result<DocumentIntegrityPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let actor = owner(&mut tx, actor)?;
        if !(1..=100).contains(&query.limit()) {
            return Err(ApplicationError::InvalidInput(
                "incident limit must be between 1 and 100".into(),
            ));
        }
        let after = query.after_id().map(|id| id.as_uuid());
        let rows = tx
            .query(
                "SELECT * FROM document_integrity_incidents
            WHERE ($1::uuid IS NULL OR id>$1) ORDER BY id LIMIT $2",
                &[&after, &(i64::from(query.limit()) + 1)],
            )
            .map_err(port_error)?;
        let mut incidents = rows
            .iter()
            .map(codec::decode)
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = incidents.len() > query.limit() as usize;
        incidents.truncate(query.limit() as usize);
        let next_after_id = if has_more {
            incidents.last().map(|incident| incident.id)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &actor,
            "document.integrity_incidents_listed",
            "document-integrity-incidents",
            at,
        )?;
        tx.commit().map_err(port_error)?;
        Ok(DocumentIntegrityPage {
            incidents,
            has_more,
            next_after_id,
        })
    }

    fn get(
        &self,
        actor: UserId,
        id: DocumentIntegrityIncidentId,
        at: OffsetDateTime,
    ) -> Result<DocumentIntegrityIncident, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let actor = owner(&mut tx, actor)?;
        let row = tx
            .query_opt(
                "SELECT * FROM document_integrity_incidents WHERE id=$1",
                &[&id.as_uuid()],
            )
            .map_err(port_error)?
            .ok_or_else(|| ApplicationError::DocumentIntegrityIncidentNotFound(id.to_string()))?;
        let incident = codec::decode(&row)?;
        append_transaction(
            &mut tx,
            &actor,
            "document.integrity_incident_read",
            &format!("document-integrity-incident:{id}"),
            at,
        )?;
        tx.commit().map_err(port_error)?;
        Ok(incident)
    }
}

fn owner(tx: &mut Transaction<'_>, actor: UserId) -> Result<String, ApplicationError> {
    let principal = active_actor(tx, actor)?;
    if principal.role != Role::Owner {
        return Err(ApplicationError::PermissionDenied);
    }
    Ok(principal.email)
}

fn receipt(incident: &DocumentIntegrityIncident) -> DocumentIntegrityReceipt {
    DocumentIntegrityReceipt {
        incident_id: incident.id,
        observation_id: incident.observation_id,
        recorded_at: incident.recorded_at,
    }
}

fn insert(
    tx: &mut Transaction<'_>,
    incident: &DocumentIntegrityIncident,
) -> Result<(), ApplicationError> {
    let capture = codec::capture(incident);
    let digest = RingSha256Hasher.hash_bytes(&capture);
    tx.execute("INSERT INTO document_integrity_incidents(id,observation_id,case_id,document_id,
        document_version,requester,failure,detected_at_seconds,detected_at_nanoseconds,
        recorded_at_seconds,recorded_at_nanoseconds,expected_digest,observed_snapshot_digest,
        capture_canonical,capture_digest) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
        &[&incident.id.as_uuid(), &incident.observation_id.as_uuid(), &incident.case_id.as_uuid(),
            &incident.reference.id.as_uuid(), &i64::from(incident.reference.version.get()),
            &incident.requester.as_uuid(), &incident.failure.as_str(),
            &incident.detected_at.unix_timestamp(), &(incident.detected_at.nanosecond() as i32),
            &incident.recorded_at.unix_timestamp(), &(incident.recorded_at.nanosecond() as i32),
            &&incident.expected_digest.as_bytes()[..], &&incident.observed_snapshot_digest.as_bytes()[..],
            &capture, &&digest.as_bytes()[..]]).map_err(port_error)?;
    Ok(())
}

pub(crate) fn validate_inventory<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let mut after: Option<uuid::Uuid> = None;
    loop {
        let rows = client
            .query(
                "SELECT i.*, EXISTS(SELECT 1 FROM documents d
            WHERE d.id=i.document_id AND d.case_id=i.case_id AND d.version=i.document_version)
            AND EXISTS(SELECT 1 FROM users u WHERE u.id=i.requester) AS references_exist
            FROM document_integrity_incidents i WHERE ($1::uuid IS NULL OR i.id>$1)
            ORDER BY i.id LIMIT 64",
                &[&after],
            )
            .map_err(port_error)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let incident = codec::decode(&row)?;
            if !row.get::<_, bool>("references_exist") {
                return Err(codec::inconsistent());
            }
            after = Some(incident.id.as_uuid());
        }
    }
    Ok(())
}
