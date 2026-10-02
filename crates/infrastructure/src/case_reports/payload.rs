use super::*;
use domain::crypto::{cipher::SealedPayload, keys::WrappedDek};
use zeroize::Zeroizing;

impl PostgresCaseReportStore {
    pub(super) fn load_snapshot(
        &self,
        tx: &mut Transaction<'_>,
        job: &storage::Job,
    ) -> Result<Option<CaseReportSnapshot>, ApplicationError> {
        let row = tx
            .query_opt(
                "SELECT * FROM case_report_snapshots WHERE report_id=$1",
                &[&job.detail.id.as_uuid()],
            )
            .map_err(port)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let protected = ProtectedCaseReportPayload {
            wrapped_dek: WrappedDek::from_bytes(row.get("wrapped_dek")).map_err(inconsistent)?,
            payload: SealedPayload::from_bytes(row.get("payload")).map_err(inconsistent)?,
        };
        let bytes = Zeroizing::new(self.protector.open(
            CaseReportProtectionContext {
                report_id: job.detail.id,
                kind: CaseReportPayloadKind::Snapshot,
                plaintext_digest: digest(row.get::<_, &[u8]>("plaintext_digest"))?,
            },
            &protected,
        )?);
        let snapshot = codec::read_snapshot(&bytes)?;
        validate_case_report_snapshot(self.hasher.as_ref(), &snapshot)?;
        let ids: Vec<_> = snapshot
            .cases
            .iter()
            .map(|row| row.case_id.as_uuid())
            .collect();
        if snapshot.report_id != job.detail.id
            || snapshot.requester != job.detail.requester
            || snapshot.scope != job.detail.scope
            || snapshot.filters != job.detail.command.filters
            || snapshot.digest != digest(row.get::<_, &[u8]>("digest"))?
            || timestamp(snapshot.checked_at)? != row.get::<_, String>("checked_at")
            || ids != job.case_ids
        {
            return Err(inconsistent(
                "encrypted capture differs from its report index",
            ));
        }
        Ok(Some(snapshot))
    }
    pub(super) fn save_snapshot(
        &self,
        tx: &mut Transaction<'_>,
        snapshot: &CaseReportSnapshot,
    ) -> Result<(), ApplicationError> {
        let bytes = Zeroizing::new(codec::snapshot(snapshot)?);
        let plain = self.hasher.hash_bytes(&bytes);
        let sealed = self.protector.seal(
            CaseReportProtectionContext {
                report_id: snapshot.report_id,
                kind: CaseReportPayloadKind::Snapshot,
                plaintext_digest: plain,
            },
            &bytes,
        )?;
        tx.execute("INSERT INTO case_report_snapshots(report_id,digest,plaintext_digest,checked_at,wrapped_dek,payload) VALUES($1,$2,$3,$4,$5,$6)",
            &[&snapshot.report_id.as_uuid(), &&snapshot.digest.as_bytes()[..], &&plain.as_bytes()[..], &timestamp(snapshot.checked_at)?,
            &sealed.wrapped_dek.as_bytes(), &sealed.payload.as_bytes()]).map_err(port)?;
        Ok(())
    }
    pub(super) fn save_artifact(
        &self,
        tx: &mut Transaction<'_>,
        artifact: &CaseReportArtifact,
    ) -> Result<(), ApplicationError> {
        let protected = self.protector.seal(
            context(artifact.report_id, artifact.format, artifact.digest),
            &artifact.content,
        )?;
        tx.execute("INSERT INTO case_report_artifacts(report_id,format,snapshot_digest,digest,bytes,wrapped_dek,payload) VALUES($1,$2,$3,$4,$5,$6,$7)",
            &[&artifact.report_id.as_uuid(), &format_name(artifact.format), &&artifact.snapshot_digest.as_bytes()[..],
            &&artifact.digest.as_bytes()[..], &(artifact.content.len() as i64), &protected.wrapped_dek.as_bytes(), &protected.payload.as_bytes()]).map_err(port)?;
        Ok(())
    }
    pub(super) fn load_artifact(
        &self,
        tx: &mut Transaction<'_>,
        job: &storage::Job,
        format: CaseReportFormat,
        snapshot: &CaseReportSnapshot,
    ) -> Result<CaseReportArtifact, ApplicationError> {
        let row = tx
            .query_opt(
                "SELECT * FROM case_report_artifacts WHERE report_id=$1 AND format=$2",
                &[&job.detail.id.as_uuid(), &format_name(format)],
            )
            .map_err(port)?
            .ok_or_else(|| inconsistent("ready artifact is missing"))?;
        let content_digest = digest(row.get::<_, &[u8]>("digest"))?;
        if snapshot.digest != digest(row.get::<_, &[u8]>("snapshot_digest"))? {
            return Err(inconsistent("artifact snapshot digest differs"));
        }
        let protected = ProtectedCaseReportPayload {
            wrapped_dek: WrappedDek::from_bytes(row.get("wrapped_dek")).map_err(inconsistent)?,
            payload: SealedPayload::from_bytes(row.get("payload")).map_err(inconsistent)?,
        };
        let content = self
            .protector
            .open(context(job.detail.id, format, content_digest), &protected)?;
        if content.len() > MAX_REPORT_ARTIFACT_BYTES
            || content.len() as i64 != row.get::<_, i64>("bytes")
            || content.is_empty()
            || self.hasher.hash_bytes(&content) != content_digest
        {
            return Err(inconsistent(
                "artifact content differs from stored metadata",
            ));
        }
        Ok(CaseReportArtifact {
            report_id: job.detail.id,
            requester: job.detail.requester.clone(),
            scope: job.detail.scope,
            format,
            snapshot_digest: snapshot.digest,
            digest: content_digest,
            content,
        })
    }
}
fn context(
    report_id: CaseReportId,
    format: CaseReportFormat,
    plaintext_digest: Sha256Digest,
) -> CaseReportProtectionContext {
    CaseReportProtectionContext {
        report_id,
        kind: match format {
            CaseReportFormat::Pdf => CaseReportPayloadKind::Pdf,
            CaseReportFormat::Csv => CaseReportPayloadKind::Csv,
        },
        plaintext_digest,
    }
}
