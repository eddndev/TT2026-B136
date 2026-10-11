use super::*;

pub(super) fn enqueue(
    tx: &mut Transaction<'_>,
    detail: &CaseReportDetail,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    if detail
        .estimate
        .is_some_and(CaseReportEstimate::requires_ready_email)
    {
        tx.execute(
            "INSERT INTO case_report_email_deliveries
            (report_id,delivery_id,state,generation,created_at,updated_at)
            VALUES($1,$2,'pending',0,$3,$3)",
            &[&detail.id.as_uuid(), &Uuid::new_v4(), &timestamp(at)?],
        )
        .map_err(port)?;
    }
    Ok(())
}

pub(super) fn state(
    tx: &mut Transaction<'_>,
    id: CaseReportId,
) -> Result<Option<CaseReportEmailState>, ApplicationError> {
    tx.query_opt(
        "SELECT state FROM case_report_email_deliveries WHERE report_id=$1",
        &[&id.as_uuid()],
    )
    .map_err(port)?
    .map(|row| email_record::state(row.get(0)))
    .transpose()
}

impl CaseReportEmailStore for PostgresCaseReportStore {
    fn claim_delivery(
        &self,
        claim: CaseReportEmailClaim,
        template: &CaseReportEmailTemplate,
    ) -> Result<Option<CaseReportEmailLease>, ApplicationError> {
        email_claim::claim(self, claim, template)
    }

    fn authorize_delivery(
        &self,
        lease: &CaseReportEmailLease,
        at: OffsetDateTime,
    ) -> Result<Option<CaseReportEmailMessage>, ApplicationError> {
        self.tx(|tx| {
            let Some(delivery) = email_record::load(tx, lease.report_id)? else {
                return Ok(None);
            };
            let now = self.observed(at)?;
            if now < delivery.updated_at {
                return Err(inconsistent("report email clock predates its state"));
            }
            if !delivery.live(lease.delivery_id, lease.generation, lease.lease_id, now)
                || delivery.lease_until != Some(lease.lease_until)
                || delivery.provider_deadline != Some(lease.provider_deadline)
            {
                return Ok(None);
            }
            let job = email_record::ready_job(tx, lease.report_id, self.hasher.as_ref())?;
            let now = self.observed(now)?;
            if !delivery.live(lease.delivery_id, lease.generation, lease.lease_id, now)
                || !email_record::authorize(tx, &job, now)?
            {
                return Ok(None);
            }
            Ok(delivery.message)
        })
    }

    fn finish_delivery(&self, result: CaseReportEmailFinish) -> Result<bool, ApplicationError> {
        if !matches!(
            result.state,
            CaseReportEmailState::Accepted
                | CaseReportEmailState::Rejected
                | CaseReportEmailState::Uncertain
        ) || (result.retry_at.is_some() && result.state != CaseReportEmailState::Uncertain)
        {
            return Err(ApplicationError::InvalidInput(
                "report email outcome is invalid".into(),
            ));
        }
        self.tx(|tx| {
            let Some(delivery) = email_record::load(tx, result.report_id)? else {
                return Ok(false);
            };
            let now = self.observed(result.at)?;
            if now < delivery.updated_at {
                return Err(inconsistent("report email clock predates its state"));
            }
            if !delivery.live(result.delivery_id, result.generation, result.lease_id, now) {
                return Ok(false);
            }
            if result.retry_at.is_some_and(|retry| {
                retry.offset() != UtcOffset::UTC
                    || !(1..=9999).contains(&retry.year())
                    || retry <= now
                    || delivery
                        .provider_deadline
                        .is_none_or(|deadline| retry >= deadline)
            }) {
                return Err(ApplicationError::InvalidInput(
                    "report email retry is outside its window".into(),
                ));
            }
            let job = email_record::ready_job(tx, result.report_id, self.hasher.as_ref())?;
            let now = self.observed(now)?;
            if !delivery.live(result.delivery_id, result.generation, result.lease_id, now) {
                return Ok(false);
            }
            if result.retry_at.is_some_and(|retry| retry <= now) {
                return Err(ApplicationError::InvalidInput(
                    "report email retry predates its commit".into(),
                ));
            }
            tx.execute(
                "UPDATE case_report_email_deliveries SET state=$2,
                lease_id=NULL,lease_until=NULL,retry_at=$3,updated_at=$4 WHERE report_id=$1",
                &[
                    &result.report_id.as_uuid(),
                    &email_record::state_name(result.state),
                    &result.retry_at.map(timestamp).transpose()?,
                    &timestamp(now)?,
                ],
            )
            .map_err(port)?;
            audit(tx, &job.detail, "case_report.email_finish", now)?;
            Ok(true)
        })
    }
}
