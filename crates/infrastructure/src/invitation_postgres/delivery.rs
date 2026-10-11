use super::{values::*, PostgresInvitationStore};
use crate::audit_postgres::begin_audited;
use application::{identity::invitations::*, ApplicationError};

impl PostgresInvitationStore {
    pub(super) fn claim_mail(
        &self,
        c: InvitationDeliveryClaim,
    ) -> Result<Option<InvitationDeliveryLease>, ApplicationError> {
        if c.lease_id.is_nil()
            || c.lease_seconds == 0
            || c.lease_seconds > 120
            || c.provider_window_seconds == 0
            || c.provider_window_seconds > 86400
        {
            return Err(ApplicationError::InvalidInput(
                "invalid invitation delivery lease".into(),
            ));
        }
        let mut client = self.lock()?;
        let mut tx = begin_audited(&mut client)?;
        let at = now(&mut tx)?;
        let row=tx.query_opt("SELECT * FROM member_invitations WHERE accepted_at IS NULL AND cancelled_at IS NULL AND expires_at>$1 AND delivery_state IN ('pending','uncertain') AND (retry_at IS NULL OR retry_at<=$1) AND (lease_until IS NULL OR lease_until<=$1) AND (provider_deadline IS NULL OR provider_deadline>$1) ORDER BY issued_at,id LIMIT 1 FOR UPDATE",&[&at]).map_err(|_|storage())?;
        let Some(row) = row else { return Ok(None) };
        let id: Uuid = row.get("id");
        let revision: i64 = row.get("revision");
        let expires: i64 = row.get("expires_at");
        let deadline = row.get::<_, Option<i64>>("provider_deadline").unwrap_or(
            at.checked_add(i64::from(c.provider_window_seconds))
                .ok_or_else(storage)?
                .min(expires),
        );
        let until = at
            .checked_add(i64::from(c.lease_seconds))
            .ok_or_else(storage)?
            .min(expires)
            .min(deadline);
        let generation = row
            .get::<_, i64>("delivery_generation")
            .checked_add(1)
            .ok_or_else(storage)?;
        tx.execute("UPDATE member_invitations SET lease_id=$2,lease_until=$3,provider_deadline=$4,delivery_generation=$5 WHERE id=$1",&[&id,&c.lease_id,&until,&deadline,&generation]).map_err(|_|storage())?;
        let result = InvitationDeliveryLease {
            id: InvitationId::from_uuid(id),
            revision: revision.try_into().map_err(|_| storage())?,
            delivery_id: row.get("operation_id"),
            generation: generation.try_into().map_err(|_| storage())?,
            lease_id: c.lease_id,
            email: row.get("email"),
            protected_token: row.get("protected_token"),
            expires_at: seconds(expires)?,
            lease_until: seconds(until)?,
            provider_deadline: seconds(deadline)?,
        };
        tx.commit().map_err(|_| storage())?;
        Ok(Some(result))
    }
    pub(super) fn finish_mail(
        &self,
        c: InvitationDeliveryFinish,
    ) -> Result<bool, ApplicationError> {
        let mut client = self.lock()?;
        let mut tx = begin_audited(&mut client)?;
        let at = now(&mut tx)?;
        let (Ok(revision), Ok(generation)) =
            (i64::try_from(c.revision), i64::try_from(c.generation))
        else {
            return Ok(false);
        };
        let row=tx.query_opt("SELECT provider_deadline FROM member_invitations WHERE id=$1 AND revision=$2 AND delivery_generation=$3 AND lease_id=$4 AND lease_until>$5 AND accepted_at IS NULL AND cancelled_at IS NULL AND expires_at>$5 FOR UPDATE",&[&c.id.as_uuid(),&revision,&generation,&c.lease_id,&at]).map_err(|_|storage())?;
        let Some(row) = row else { return Ok(false) };
        let deadline: i64 = row.get("provider_deadline");
        let retry = c
            .retry_at
            .map(|t| t.unix_timestamp().max(at))
            .filter(|t| *t < deadline);
        let state = match c.state {
            InvitationDeliveryState::Sent => "sent",
            InvitationDeliveryState::Rejected => "rejected",
            InvitationDeliveryState::Pending => "pending",
            InvitationDeliveryState::Uncertain => "uncertain",
        };
        let next = if matches!(
            c.state,
            InvitationDeliveryState::Pending | InvitationDeliveryState::Uncertain
        ) {
            retry.or(Some(deadline))
        } else {
            None
        };
        tx.execute("UPDATE member_invitations SET delivery_state=$2,retry_at=$3,lease_id=NULL,lease_until=NULL WHERE id=$1",&[&c.id.as_uuid(),&state,&next]).map_err(|_|storage())?;
        tx.commit().map_err(|_| storage())?;
        Ok(true)
    }
}
