//! Leased e-mail delivery of contract notices. Only the delivery columns
//! change, under the storage clock; the notice content stays immutable.

use super::{errors::port, PostgresContractStore};
use application::{
    case_reports::CaseReportEmailMessage,
    contracts::{
        ContractNoticeClaim, ContractNoticeFinish, ContractNoticeLease, ContractNoticeState,
        ContractNoticeStore,
    },
    ApplicationError,
};
use domain::clock::OffsetDateTime;
use postgres::{IsolationLevel, Transaction};

impl ContractNoticeStore for PostgresContractStore {
    fn claim_notice(
        &self,
        claim: ContractNoticeClaim,
    ) -> Result<Option<ContractNoticeLease>, ApplicationError> {
        if claim.lease_id.is_nil()
            || !(1..=120).contains(&claim.lease_seconds)
            || !(1..=86_400).contains(&claim.provider_window_seconds)
        {
            return Err(ApplicationError::InvalidInput(
                "invalid contract notice lease".into(),
            ));
        }
        let mut client = self.client()?;
        let mut tx = begin(&mut client)?;
        let at = now(&mut tx)?;
        let row = tx
            .query_opt(
                "SELECT n.id,n.delivery_generation,n.provider_deadline,n.message_from,n.message_to,
                    n.subject,n.message_text FROM contract_notifications n
                WHERE n.delivery_state IN ('pending','uncertain')
                    AND (n.retry_at IS NULL OR n.retry_at<=$1)
                    AND (n.lease_until IS NULL OR n.lease_until<=$1)
                    AND (n.provider_deadline IS NULL OR n.provider_deadline>$1)
                    AND EXISTS(SELECT 1 FROM users u WHERE u.id=n.recipient_id AND u.active)
                ORDER BY n.created_at_seconds,n.id LIMIT 1 FOR UPDATE OF n SKIP LOCKED",
                &[&at],
            )
            .map_err(port)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let deadline = row
            .get::<_, Option<i64>>("provider_deadline")
            .unwrap_or(at + i64::from(claim.provider_window_seconds));
        let until = (at + i64::from(claim.lease_seconds)).min(deadline);
        let generation = row
            .get::<_, i64>("delivery_generation")
            .checked_add(1)
            .ok_or_else(storage)?;
        let id: uuid::Uuid = row.get("id");
        tx.execute(
            "UPDATE contract_notifications SET lease_id=$2,lease_until=$3,provider_deadline=$4,
                delivery_generation=$5 WHERE id=$1",
            &[&id, &claim.lease_id, &until, &deadline, &generation],
        )
        .map_err(port)?;
        let lease = ContractNoticeLease {
            id,
            generation: u64::try_from(generation).map_err(|_| storage())?,
            lease_id: claim.lease_id,
            lease_until: seconds(until)?,
            provider_deadline: seconds(deadline)?,
            message: CaseReportEmailMessage {
                from: row.get("message_from"),
                to: row.get("message_to"),
                subject: row.get("subject"),
                text: row.get("message_text"),
            },
        };
        tx.commit().map_err(port)?;
        Ok(Some(lease))
    }

    fn finish_notice(&self, finish: ContractNoticeFinish) -> Result<bool, ApplicationError> {
        let Ok(generation) = i64::try_from(finish.generation) else {
            return Ok(false);
        };
        let mut client = self.client()?;
        let mut tx = begin(&mut client)?;
        let at = now(&mut tx)?;
        let row = tx
            .query_opt(
                "SELECT provider_deadline FROM contract_notifications WHERE id=$1
                    AND delivery_generation=$2 AND lease_id=$3 AND lease_until>$4 FOR UPDATE",
                &[&finish.id, &generation, &finish.lease_id, &at],
            )
            .map_err(port)?;
        let Some(row) = row else {
            return Ok(false);
        };
        let deadline: i64 = row.get("provider_deadline");
        let (state, retry) = match finish.state {
            ContractNoticeState::Sent => ("sent", None),
            ContractNoticeState::Rejected => ("rejected", None),
            // An uncertain attempt without a retry stays uncertain for good:
            // its retry time is the deadline, after which no claim matches.
            ContractNoticeState::Uncertain => (
                "uncertain",
                Some(
                    finish
                        .retry_at
                        .map(|retry| retry.unix_timestamp().max(at))
                        .filter(|retry| *retry < deadline)
                        .unwrap_or(deadline),
                ),
            ),
        };
        tx.execute(
            "UPDATE contract_notifications SET delivery_state=$2,retry_at=$3,lease_id=NULL,
                lease_until=NULL WHERE id=$1",
            &[&finish.id, &state, &retry],
        )
        .map_err(port)?;
        tx.commit().map_err(port)?;
        Ok(true)
    }
}

fn begin<'a>(client: &'a mut postgres::Client) -> Result<Transaction<'a>, ApplicationError> {
    client
        .build_transaction()
        .isolation_level(IsolationLevel::ReadCommitted)
        .start()
        .map_err(port)
}

fn now(tx: &mut Transaction<'_>) -> Result<i64, ApplicationError> {
    tx.query_one(
        "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
        &[],
    )
    .map(|row| row.get(0))
    .map_err(port)
}

fn seconds(at: i64) -> Result<OffsetDateTime, ApplicationError> {
    OffsetDateTime::from_unix_timestamp(at).map_err(|_| storage())
}

fn storage() -> ApplicationError {
    ApplicationError::Port("contract notice storage operation failed".into())
}
