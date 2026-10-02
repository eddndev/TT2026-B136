use super::*;
use application::cases::{CaseStatusFilter, CurrentCaseAdministration};
use domain::{case_administration::CaseAdministrativeStatus, cases::CaseId, identity::UserId};
use std::collections::BTreeMap;

impl PostgresCaseReportStore {
    pub(super) fn capture_report(
        &self,
        lease: &CaseReportLease,
        at: OffsetDateTime,
    ) -> Result<CaseReportSnapshot, ApplicationError> {
        self.tx(|tx| {
            let job = storage::get(tx, lease.report_id, self.hasher.as_ref())?;
            let now = self.observed(at)?;
            storage::fence(&job, lease, now)?;
            access::original(tx, &job)?;
            if let Some(saved) = self.load_snapshot(tx, &job)? { return Ok(saved); }
            if job.detail.state != CaseReportState::Processing(CaseReportPhase::Capturing) {
                return Err(inconsistent("rendering report is missing its immutable capture"));
            }
            let snapshot = self.build_snapshot(tx, &job.detail, now)?;
            self.save_snapshot(tx, &snapshot)?;
            let committed_at = self.observed(now)?;
            storage::fence(&job, lease, committed_at)?;
            let ids: Vec<_> = snapshot.cases.iter().map(|row| row.case_id.as_uuid()).collect();
            tx.execute("UPDATE case_report_jobs SET state='rendering',case_ids=$2,updated_at=$3 WHERE id=$1",
                &[&job.detail.id.as_uuid(), &ids, &timestamp(committed_at)?]).map_err(port)?;
            audit(tx, &job.detail, "case_report.capture", committed_at)?;
            Ok(snapshot)
        })
    }
    fn build_snapshot(
        &self,
        tx: &mut Transaction<'_>,
        report: &CaseReportDetail,
        now: OffsetDateTime,
    ) -> Result<CaseReportSnapshot, ApplicationError> {
        let f = &report.command.filters;
        access::filter_member(tx, &report.requester.principal, f.assigned_litigator)?;
        let status = match f.status {
            CaseStatusFilter::All => "all",
            CaseStatusFilter::Active => "active",
            CaseStatusFilter::Closed => "closed",
        };
        let rows = tx.query("SELECT c.id FROM cases c LEFT JOIN LATERAL
            (SELECT administrative_status FROM case_administration_revisions r WHERE r.case_id=c.id ORDER BY revision DESC LIMIT 1) h ON true
            WHERE c.created_at >= $1::text::timestamptz AND c.created_at < $2::text::timestamptz
            AND c.created_at <= $3::text::timestamptz
            AND ($4::boolean OR EXISTS(SELECT 1 FROM case_memberships m WHERE m.case_id=c.id AND m.user_id=$5))
            AND ($6='all' OR coalesce(h.administrative_status,'active')=$6)
            AND ($7::uuid IS NULL OR EXISTS(SELECT 1 FROM case_memberships m JOIN users u ON u.id=m.user_id
                WHERE m.case_id=c.id AND u.id=$7 AND u.active AND u.role='litigator'))
            ORDER BY c.id LIMIT $8", &[&timestamp(f.created_from)?, &timestamp(f.created_before)?, &timestamp(now)?,
            &(report.scope == CaseReportScope::Office), &report.requester.principal.id.as_uuid(), &status,
            &f.assigned_litigator.map(UserId::as_uuid), &((MAX_REPORT_CASES + 1) as i64)]).map_err(port)?;
        if rows.len() > MAX_REPORT_CASES {
            return Err(CaseReportError::CapacityExceeded.into());
        }
        let ids: Vec<Uuid> = rows.iter().map(|row| row.get(0)).collect();
        let assignments = tx.query("SELECT m.case_id,u.id,u.email FROM case_memberships m JOIN users u ON u.id=m.user_id
            WHERE m.case_id=ANY($1::uuid[]) AND u.active AND u.role='litigator' ORDER BY m.case_id,u.id LIMIT $2 FOR SHARE OF m,u",
            &[&ids, &((MAX_REPORT_ASSIGNMENTS + 1) as i64)]).map_err(port)?;
        if assignments.len() > MAX_REPORT_ASSIGNMENTS {
            return Err(CaseReportError::CapacityExceeded.into());
        }
        let mut members: BTreeMap<Uuid, Vec<CaseReportLitigator>> = BTreeMap::new();
        for row in assignments {
            members
                .entry(row.get("case_id"))
                .or_default()
                .push(CaseReportLitigator {
                    user_id: UserId::from_uuid(row.get("id")),
                    email: row.get("email"),
                });
        }
        let mut cases = Vec::with_capacity(ids.len());
        let mut workloads: BTreeMap<Uuid, CaseReportWorkload> = BTreeMap::new();
        for id in ids {
            let detail =
                crate::cases::storage::detail(tx, CaseId::from_uuid(id), self.hasher.as_ref())?;
            let values = detail.administration.values();
            let assigned_litigators = members.remove(&id).unwrap_or_default();
            for member in &assigned_litigators {
                if !workloads.contains_key(&member.user_id.as_uuid())
                    && workloads.len() >= MAX_REPORT_WORKLOAD
                {
                    return Err(CaseReportError::CapacityExceeded.into());
                }
                let workload = workloads
                    .entry(member.user_id.as_uuid())
                    .or_insert_with(|| CaseReportWorkload {
                        litigator: member.clone(),
                        active_cases: 0,
                        closed_cases: 0,
                    });
                if values.status() == CaseAdministrativeStatus::Active {
                    workload.active_cases += 1;
                } else {
                    workload.closed_cases += 1;
                }
            }
            let (revision, digest) = match &detail.administration {
                CurrentCaseAdministration::Unrevised(_) => (None, None),
                CurrentCaseAdministration::Recorded(value) => {
                    (Some(value.revision), Some(value.values_digest))
                }
            };
            cases.push(CaseReportRow {
                case_id: detail.origin.id,
                title: values.metadata().title().into(),
                reference: values.metadata().reference().into(),
                created_at: detail.origin.created_at,
                status: values.status(),
                administration_revision: revision,
                administration_digest: digest,
                assigned_litigators,
            });
        }
        let mut snapshot = CaseReportSnapshot {
            report_id: report.id,
            requester: report.requester.clone(),
            scope: report.scope,
            filters: f.clone(),
            checked_at: now,
            cases,
            workload: workloads.into_values().collect(),
            digest: Sha256Digest::from_array([0; 32]),
        };
        snapshot.digest = case_report_snapshot_digest(self.hasher.as_ref(), &snapshot)?;
        Ok(snapshot)
    }
}
