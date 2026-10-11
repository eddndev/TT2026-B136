use super::{values::*, PostgresInvitationStore};
use crate::audit_postgres::begin_audited;
use application::{identity::invitations::*, ApplicationError};
use domain::crypto::Sha256Digest;

impl PostgresInvitationStore {
    pub(super) fn read_list(
        &self,
        actor: InvitationActor,
        query: InvitationQuery,
    ) -> Result<InvitationPage, ApplicationError> {
        if query.limit == 0 || query.limit > 100 {
            return Err(ApplicationError::InvalidInput(
                "invitation page limit must be 1..100".into(),
            ));
        }
        let mut client = self.lock()?;
        let mut tx = begin_audited(&mut client)?;
        authority(&mut tx, &actor)?;
        let at = now(&mut tx)?;
        let after = query.after.map(|id| id.as_uuid());
        let limit = i64::from(query.limit) + 1;
        let rows=tx.query("SELECT * FROM member_invitations WHERE ($1::uuid IS NULL OR id>$1) ORDER BY id LIMIT $2",&[&after,&limit]).map_err(|_|storage())?;
        let more = rows.len() > usize::from(query.limit);
        let items = rows
            .iter()
            .take(usize::from(query.limit))
            .map(|r| summary(r, at))
            .collect::<Result<Vec<_>, _>>()?;
        let next = if more {
            items.last().map(|s| s.id)
        } else {
            None
        };
        tx.commit().map_err(|_| storage())?;
        Ok(InvitationPage { items, next })
    }
    pub(super) fn read_get(
        &self,
        actor: InvitationActor,
        id: InvitationId,
    ) -> Result<InvitationSummary, ApplicationError> {
        let mut client = self.lock()?;
        let mut tx = begin_audited(&mut client)?;
        authority(&mut tx, &actor)?;
        let at = now(&mut tx)?;
        let row = tx
            .query_opt(
                "SELECT * FROM member_invitations WHERE id=$1",
                &[&id.as_uuid()],
            )
            .map_err(|_| storage())?
            .ok_or(ApplicationError::UserNotFound)?;
        let result = summary(&row, at)?;
        tx.commit().map_err(|_| storage())?;
        Ok(result)
    }
    pub(super) fn read_candidate(
        &self,
        hash: Sha256Digest,
    ) -> Result<Option<InvitationCandidate>, ApplicationError> {
        let row=self.lock()?.query_opt("SELECT * FROM member_invitations WHERE token_digest=$1 AND accepted_at IS NULL AND cancelled_at IS NULL AND expires_at>floor(extract(epoch FROM clock_timestamp())) AND NOT EXISTS(SELECT 1 FROM users WHERE users.email=member_invitations.email)",&[&&hash.as_bytes()[..]]).map_err(|_|storage())?;
        row.map(|r| {
            Ok(InvitationCandidate {
                id: InvitationId::from_uuid(r.get("id")),
                revision: u64::try_from(r.get::<_, i64>("revision")).map_err(|_| storage())?,
                email: r.get("email"),
                role: r.get::<_, String>("role").parse()?,
                expires_at: seconds(r.get("expires_at"))?,
                pending: pending(&r)?,
            })
        })
        .transpose()
    }
    pub(super) fn read_completed(
        &self,
        hash: Sha256Digest,
        op: Uuid,
    ) -> Result<Option<InvitationCompletion>, ApplicationError> {
        let row=self.lock()?.query_opt("SELECT receipt,request_digest FROM member_invitation_completions WHERE token_digest=$1 AND operation_id=$2",&[&&hash.as_bytes()[..],&op]).map_err(|_|storage())?;
        row.map(|r| {
            Ok(InvitationCompletion {
                receipt: decode_receipt(r.get("receipt"))?,
                request_digest: digest(r.get("request_digest"))?,
            })
        })
        .transpose()
    }
}
