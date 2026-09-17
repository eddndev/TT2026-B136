use application::ApplicationError;
use domain::clock::OffsetDateTime;
use postgres::{Client, GenericClient, NoTls};

use crate::audit_postgres::{append_transaction, begin_audited};
use crate::documents::encode_evidence;

use super::{invalid, LegacyImport};

impl LegacyImport {
    /// Checks destination associations and history without DDL or writes.
    pub fn check_target(&self, database_url: &str) -> Result<(), ApplicationError> {
        let mut client = Client::connect(database_url, NoTls).map_err(invalid)?;
        let mut transaction = client
            .build_transaction()
            .read_only(true)
            .start()
            .map_err(invalid)?;
        self.inspect_target(&mut transaction)?;
        transaction.rollback().map_err(invalid)
    }

    /// Imports one validated snapshot atomically and makes its files read-only.
    /// A retry reconciles the receipt and bytes before recreating missing markers.
    pub fn apply(&self, database_url: &str) -> Result<(), ApplicationError> {
        let _source_locks = self.lock_source()?;
        let mut client = Client::connect(database_url, NoTls).map_err(invalid)?;
        let mut transaction = begin_audited(&mut client)?;
        let imported = self.inspect_target(&mut transaction)?;
        self.prepare_source()?;
        if !imported {
            for (case_id, record) in &self.records {
                let evidence = record.evidence.as_ref().map(encode_evidence).transpose()?;
                transaction.execute(
                    "INSERT INTO document_series(id,case_id,first_available_version) VALUES($1,$2,$3)",
                    &[&record.id.as_uuid(), &case_id.as_uuid(), &i64::from(record.version.get())],
                ).map_err(invalid)?;
                transaction
                    .execute(
                        "INSERT INTO documents(id,case_id,version,name,digest,vault,evidence)
                     VALUES($1,$2,$3,$4,$5,$6,$7)",
                        &[
                            &record.id.as_uuid(),
                            &case_id.as_uuid(),
                            &i64::from(record.version.get()),
                            &record.name,
                            &&record.digest.as_bytes()[..],
                            &record.vault,
                            &evidence,
                        ],
                    )
                    .map_err(invalid)?;
            }
            for entry in &self.entries {
                let sequence = i64::try_from(entry.event.sequence).map_err(invalid)?;
                let at = entry.event.timestamp_rfc3339()?;
                transaction
                    .execute(
                        "INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain)
                     VALUES($1,$2,$3,$4,$5,$6)",
                        &[
                            &sequence,
                            &at,
                            &entry.event.actor,
                            &entry.event.action,
                            &entry.event.resource,
                            &&entry.chain.as_bytes()[..],
                        ],
                    )
                    .map_err(invalid)?;
            }
            append_transaction(
                &mut transaction,
                "migration",
                "migration.imported",
                &self.report.fingerprint,
                OffsetDateTime::now_utc(),
            )?;
            transaction.execute(
                "INSERT INTO migration_receipts(fingerprint,document_count,audit_count,audit_head)
                 VALUES($1,$2,$3,$4)", &[&self.report.fingerprint, &(self.report.documents as i64),
                 &(self.report.audit_entries as i64), &self.report.audit_head],
            ).map_err(invalid)?;
        }
        transaction.commit().map_err(invalid)?;
        self.mark_source().map_err(|error| ApplicationError::Port(format!(
            "database import committed; source remains fenced; repair filesystem and retry the same import: {error}")))
    }

    fn inspect_target<C: GenericClient>(&self, client: &mut C) -> Result<bool, ApplicationError> {
        for (case_id, _) in &self.records {
            if client
                .query_opt("SELECT id FROM cases WHERE id=$1", &[&case_id.as_uuid()])
                .map_err(invalid)?
                .is_none()
            {
                return Err(invalid("mapped case does not exist in destination"));
            }
        }
        let receipt = client.query_opt("SELECT document_count,audit_count,audit_head FROM migration_receipts WHERE fingerprint=$1",
            &[&self.report.fingerprint]).map_err(invalid)?;
        if let Some(receipt) = receipt {
            if receipt.get::<_, i64>(0) != self.report.documents as i64
                || receipt.get::<_, i64>(1) != self.report.audit_entries as i64
                || receipt.get::<_, String>(2) != self.report.audit_head
            {
                return Err(invalid("destination receipt contradicts source"));
            }
            self.reconcile(client)?;
            return Ok(true);
        }
        let occupied: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM document_series) OR EXISTS(SELECT 1 FROM documents) OR EXISTS(SELECT 1 FROM document_metadata_revisions) OR EXISTS(SELECT 1 FROM audit_events)
             OR EXISTS(SELECT 1 FROM case_participants) OR EXISTS(SELECT 1 FROM case_participant_revisions)
             OR EXISTS(SELECT 1 FROM case_administration_revisions) OR EXISTS(SELECT 1 FROM case_initial_stage_registrations)
             OR EXISTS(SELECT 1 FROM case_stage_revisions)
             OR EXISTS(SELECT 1 FROM case_hearings) OR EXISTS(SELECT 1 FROM case_hearing_revisions)
             OR EXISTS(SELECT 1 FROM case_hearing_results) OR EXISTS(SELECT 1 FROM case_hearing_result_revisions)
             OR EXISTS(SELECT 1 FROM case_procedural_facts) OR EXISTS(SELECT 1 FROM case_procedural_fact_revisions)
             OR EXISTS(SELECT 1 FROM judicial_calendars) OR EXISTS(SELECT 1 FROM judicial_calendar_revisions)
             OR EXISTS(SELECT 1 FROM deadline_profiles) OR EXISTS(SELECT 1 FROM deadline_profile_revisions)
             OR EXISTS(SELECT 1 FROM deadline_source_events)
             OR EXISTS(SELECT 1 FROM case_subjects) OR EXISTS(SELECT 1 FROM case_subject_revisions)
             OR EXISTS(SELECT 1 FROM case_participant_typed_revisions) OR EXISTS(SELECT 1 FROM subject_identity_reviews)
             OR EXISTS(SELECT 1 FROM participant_identity_reviews) OR EXISTS(SELECT 1 FROM participant_credential_evidence)
             OR EXISTS(SELECT 1 FROM participant_credential_authority) OR EXISTS(SELECT 1 FROM participant_credential_trust_revisions)
             OR EXISTS(SELECT 1 FROM migration_receipts)",
                &[],
            )
            .map_err(invalid)?
            .get(0);
        if occupied {
            return Err(invalid("initial import requires empty document, participant, credential trust, case administration, hearing, hearing result, judicial calendar, procedural fact, deadline profile, deadline source event and audit stores; stop writers before cutover"));
        }
        Ok(false)
    }

    fn reconcile<C: GenericClient>(&self, client: &mut C) -> Result<(), ApplicationError> {
        super::reconciliation::reconcile(
            client,
            &self.records,
            &self.entries,
            &self.report.fingerprint,
        )
    }
}
