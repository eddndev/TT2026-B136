use super::*;

impl CaseReportStore for PostgresCaseReportStore {
    fn litigators(
        &self,
        actor: &Principal,
        query: CaseReportLitigatorQuery,
        at: OffsetDateTime,
    ) -> Result<CaseReportLitigatorPage, ApplicationError> {
        self.tx(|tx| {
            access::actor(tx, actor)?;
            if !(1..=100).contains(&query.limit)
                || query.after_id.is_some_and(|id| id.as_uuid().is_nil())
            {
                return Err(ApplicationError::InvalidInput(
                    "report litigator page requires a limit from 1 to 100 and a valid cursor"
                        .into(),
                ));
            }
            let office = actor.role == domain::identity::Role::Owner;
            let sql = format!(
                "SELECT u.id,u.email FROM users u
                WHERE {} AND ($3::uuid IS NULL OR u.id>$3)
                ORDER BY u.id LIMIT $4 FOR SHARE OF u",
                access::LITIGATOR_VISIBILITY
            );
            let rows = tx
                .query(
                    &sql,
                    &[
                        &office,
                        &actor.id.as_uuid(),
                        &query.after_id.map(|id| id.as_uuid()),
                        &(i64::from(query.limit) + 1),
                    ],
                )
                .map_err(port)?;
            let has_more = rows.len() > query.limit as usize;
            let litigators: Vec<_> = rows
                .into_iter()
                .take(query.limit as usize)
                .map(|row| CaseReportLitigator {
                    user_id: domain::identity::UserId::from_uuid(row.get("id")),
                    email: row.get("email"),
                })
                .collect();
            let checked_at = self.observed(at)?;
            crate::audit_postgres::append_transaction(
                tx,
                &actor.email,
                "case_report.read",
                "case-reports:litigators",
                checked_at,
            )?;
            Ok(CaseReportLitigatorPage {
                scope: if office {
                    CaseReportScope::Office
                } else {
                    CaseReportScope::AssignedCases
                },
                checked_at,
                next_after_id: if has_more {
                    litigators.last().map(|value| value.user_id)
                } else {
                    None
                },
                litigators,
                has_more,
            })
        })
    }
    fn request(
        &self,
        actor: &Principal,
        scope: CaseReportScope,
        command: CaseReportCommand,
        request_digest: Sha256Digest,
        at: OffsetDateTime,
    ) -> Result<CaseReportDetail, ApplicationError> {
        self.request_report(actor, scope, command, request_digest, at)
    }
    fn list(
        &self,
        actor: &Principal,
        query: CaseReportQuery,
        at: OffsetDateTime,
    ) -> Result<CaseReportPage, ApplicationError> {
        self.tx(|tx| {
            let current = access::actor(tx, actor)?;
            if !(1..=100).contains(&query.limit) { return Err(ApplicationError::InvalidInput("invalid report page limit".into())); }
            let now = self.observed(at)?;
            let rows = tx.query("SELECT j.* FROM case_report_jobs j WHERE requester_id=$1 AND principal=$2
                AND account_revision<=$3 AND auth_generation=$4 AND state<>'revoked' AND ($5::uuid IS NULL OR j.id>$5)
                AND (NOT $6::boolean OR EXISTS(SELECT 1 FROM case_report_notices n WHERE n.report_id=j.id AND n.read_at IS NULL))
                AND NOT EXISTS(SELECT 1 FROM unnest(j.case_ids) id WHERE NOT EXISTS(SELECT 1 FROM cases c WHERE c.id=id))
                AND (scope='office' OR NOT EXISTS(SELECT 1 FROM unnest(j.case_ids) id
                    WHERE NOT EXISTS(SELECT 1 FROM case_memberships m WHERE m.case_id=id AND m.user_id=$1)))
                ORDER BY j.id LIMIT $7", &[&actor.id.as_uuid(), &serde_json::to_value(actor).map_err(inconsistent)?,
                &(current.account_revision as i64), &(current.auth_generation as i64), &query.after_id.map(CaseReportId::as_uuid),
                &query.unread_only, &(i64::from(query.limit) + 1)]).map_err(port)?;
            let has_more = rows.len() > query.limit as usize;
            let mut reports = Vec::with_capacity(rows.len().min(query.limit as usize));
            for row in rows.into_iter().take(query.limit as usize) {
                let job = storage::decode(tx, &row, self.hasher.as_ref())?;
                access::original(tx, &job)?;
                reports.push(job.detail);
            }
            crate::audit_postgres::append_transaction(tx, &actor.email, "case_report.read", "case-reports:list", now)?;
            Ok(CaseReportPage { checked_at: now, next_after_id: if has_more { reports.last().map(|r| r.id) } else { None }, reports, has_more })
        })
    }
    fn get(
        &self,
        actor: &Principal,
        id: CaseReportId,
        at: OffsetDateTime,
    ) -> Result<CaseReportDetail, ApplicationError> {
        self.tx(|tx| {
            access::actor(tx, actor)?;
            let now = self.observed(at)?;
            let job = self.own(tx, actor, id)?;
            audit(tx, &job.detail, "case_report.read", now)?;
            Ok(job.detail)
        })
    }
    fn download(
        &self,
        actor: &Principal,
        id: CaseReportId,
        format: CaseReportFormat,
        at: OffsetDateTime,
    ) -> Result<CaseReportDownload, ApplicationError> {
        self.tx(|tx| {
            access::actor(tx, actor)?;
            let now = self.observed(at)?;
            let job = self.own(tx, actor, id)?;
            if !matches!(job.detail.state, CaseReportState::Ready { .. }) {
                return Err(CaseReportError::NotReady.into());
            }
            let snapshot = self
                .load_snapshot(tx, &job)?
                .ok_or_else(|| inconsistent("ready snapshot is missing"))?;
            let artifact = self.load_artifact(tx, &job, format, &snapshot)?;
            audit(tx, &job.detail, "case_report.download", now)?;
            Ok(CaseReportDownload {
                report: job.detail,
                artifact,
            })
        })
    }
    fn acknowledge_notice(
        &self,
        actor: &Principal,
        id: CaseReportId,
        at: OffsetDateTime,
    ) -> Result<CaseReportDetail, ApplicationError> {
        self.tx(|tx| {
            access::actor(tx, actor)?;
            let now = self.observed(at)?;
            let job = self.own(tx, actor, id)?;
            let notice = job
                .detail
                .notice
                .as_ref()
                .ok_or(CaseReportError::NotReady)?;
            if notice.read_at.is_some() {
                return Ok(job.detail);
            }
            if now < job.detail.updated_at {
                return Err(inconsistent("notice acknowledgement clock moved backwards"));
            }
            tx.execute(
                "UPDATE case_report_notices SET read_at=$2 WHERE report_id=$1",
                &[&id.as_uuid(), &timestamp(now)?],
            )
            .map_err(port)?;
            tx.execute(
                "UPDATE case_report_jobs SET updated_at=$2 WHERE id=$1",
                &[&id.as_uuid(), &timestamp(now)?],
            )
            .map_err(port)?;
            audit(tx, &job.detail, "case_report.notice_read", now)?;
            Ok(storage::get(tx, id, self.hasher.as_ref())?.detail)
        })
    }
}
impl PostgresCaseReportStore {
    fn own(
        &self,
        tx: &mut Transaction<'_>,
        actor: &Principal,
        id: CaseReportId,
    ) -> Result<storage::Job, ApplicationError> {
        let job = storage::get(tx, id, self.hasher.as_ref())?;
        if job.detail.requester.principal.id != actor.id {
            return Err(CaseReportError::NotFound.into());
        }
        access::original(tx, &job)?;
        Ok(job)
    }
}
