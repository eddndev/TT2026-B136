use super::*;
use time::Duration;

pub(super) fn claim(
    store: &PostgresCaseReportStore,
    claim: CaseReportEmailClaim,
    template: &CaseReportEmailTemplate,
) -> Result<Option<CaseReportEmailLease>, ApplicationError> {
    if claim.lease_id.is_nil()
        || !(1..=30).contains(&claim.lease_seconds)
        || !(1..=86_400).contains(&claim.provider_window_seconds)
    {
        return Err(ApplicationError::InvalidInput(
            "report email claim limits are invalid".into(),
        ));
    }
    store.tx(|tx| {
        let now = store.observed(claim.at)?;
        let row = tx
            .query_opt(
                "SELECT report_id FROM case_report_email_deliveries
            WHERE (state='pending' OR (state='uncertain' AND retry_at IS NOT NULL
                AND retry_at::timestamptz <= $1::text::timestamptz))
            AND (lease_until IS NULL OR lease_until::timestamptz <= $1::text::timestamptz)
            ORDER BY created_at::timestamptz,report_id LIMIT 1 FOR UPDATE",
                &[&timestamp(now)?],
            )
            .map_err(port)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let id = CaseReportId::from_uuid(row.get(0));
        let delivery = email_record::load(tx, id)?
            .ok_or_else(|| inconsistent("report email intent disappeared"))?;
        let job = email_record::ready_job(tx, id, store.hasher.as_ref())?;
        let now = store.observed(now)?;
        if now < delivery.updated_at {
            return Err(inconsistent("report email clock predates its state"));
        }
        if delivery
            .provider_deadline
            .is_some_and(|deadline| deadline <= now)
        {
            tx.execute(
                "UPDATE case_report_email_deliveries SET state='uncertain',
                lease_id=NULL,lease_until=NULL,retry_at=NULL,updated_at=$2 WHERE report_id=$1",
                &[&id.as_uuid(), &timestamp(now)?],
            )
            .map_err(port)?;
            audit(tx, &job.detail, "case_report.email_finish", now)?;
            return Ok(None);
        }
        if !delivery.due(now) || !email_record::authorize(tx, &job, now)? {
            return Ok(None);
        }
        let message = match delivery.message {
            Some(message) => message,
            None => {
                let message = CaseReportEmailMessage {
                    from: template.from.clone(),
                    to: job.detail.requester.principal.email.clone(),
                    subject: template.subject.clone(),
                    text: template.text.clone(),
                };
                if !email_record::valid_message(&message) {
                    return Err(ApplicationError::InvalidInput(
                        "report email template is invalid".into(),
                    ));
                }
                message
            }
        };
        let deadline = match delivery.provider_deadline {
            Some(deadline) => deadline,
            None => now
                .checked_add(Duration::seconds(i64::from(claim.provider_window_seconds)))
                .ok_or_else(|| inconsistent("report email deadline exhausted"))?,
        };
        let until = now
            .checked_add(Duration::seconds(i64::from(claim.lease_seconds)))
            .ok_or_else(|| inconsistent("report email lease exhausted"))?
            .min(deadline);
        let generation = delivery
            .generation
            .checked_add(1)
            .filter(|value| *value <= i64::MAX as u64)
            .ok_or_else(|| inconsistent("report email generation exhausted"))?;
        tx.execute(
            "UPDATE case_report_email_deliveries SET state='pending',generation=$2,
            lease_id=$3,lease_until=$4,provider_deadline=$5,retry_at=NULL,message=$6,updated_at=$7
            WHERE report_id=$1",
            &[
                &id.as_uuid(),
                &(generation as i64),
                &claim.lease_id,
                &timestamp(until)?,
                &timestamp(deadline)?,
                &email_record::message_json(&message),
                &timestamp(now)?,
            ],
        )
        .map_err(port)?;
        audit(tx, &job.detail, "case_report.email_claim", now)?;
        Ok(Some(CaseReportEmailLease {
            report_id: id,
            delivery_id: delivery.delivery_id,
            generation,
            lease_id: claim.lease_id,
            lease_until: until,
            provider_deadline: deadline,
            message,
        }))
    })
}
