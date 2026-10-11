use super::{values::*, PostgresInvitationStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{identity::invitations::*, ApplicationError};
use postgres::{Row, Transaction};
use serde_json::{json, Value};

impl PostgresInvitationStore {
    pub(super) fn write_issue(
        &self,
        actor: InvitationActor,
        c: IssueInvitation,
    ) -> Result<InvitationSummary, ApplicationError> {
        let mut client = self.lock()?;
        let mut tx = begin_audited(&mut client)?;
        authority(&mut tx, &actor)?;
        let command =
            json!({"action":"issue","email":c.request.email,"role":c.request.role.as_str()});
        if let Some(receipt) = replay(&mut tx, &actor, c.request.operation_id, &command)? {
            return Ok(receipt);
        }
        account_absent(&mut tx, &c.request.email)?;
        if tx
            .query_opt(
                "SELECT id FROM member_invitations WHERE email=$1",
                &[&c.request.email],
            )
            .map_err(|_| storage())?
            .is_some()
        {
            return Err(ApplicationError::ConcurrentModification);
        }
        if c.request.operation_id.is_nil() {
            return Err(ApplicationError::InvalidInput(
                "invitation operation is required".into(),
            ));
        }
        let at = now(&mut tx)?;
        let until = at.checked_add(INVITATION_TTL_SECONDS).ok_or_else(storage)?;
        let id = c.request.operation_id;
        let audit = append_transaction(
            &mut tx,
            &actor.principal.email,
            "identity.invitation_issued",
            &format!("invitation:{id}:revision:1:operation:{id}"),
            seconds(at)?,
        )?;
        let seq = i64::try_from(audit.event.sequence).map_err(|_| storage())?;
        let row=tx.query_one("INSERT INTO member_invitations(id,email,role,revision,operation_id,issued_at,expires_at,token_digest,protected_token,delivery_state,audit_sequence) VALUES($1,$2,$3,1,$1,$4,$5,$6,$7,'pending',$8) RETURNING *",&[&id,&c.request.email,&c.request.role.as_str(),&at,&until,&&c.token_digest.as_bytes()[..],&c.protected_token,&seq]).map_err(|_|storage())?;
        let result = summary(&row, at)?;
        save_receipt(&mut tx, &actor, &result, command, seq)?;
        tx.commit().map_err(|_| storage())?;
        Ok(result)
    }
    pub(super) fn write_resend(
        &self,
        actor: InvitationActor,
        id: InvitationId,
        c: ResendInvitation,
    ) -> Result<InvitationSummary, ApplicationError> {
        let mut client = self.lock()?;
        let mut tx = begin_audited(&mut client)?;
        authority(&mut tx, &actor)?;
        let command = json!({"action":"resend","id":id.as_uuid(),"expected_revision":c.request.expected_revision});
        if let Some(receipt) = replay(&mut tx, &actor, c.request.operation_id, &command)? {
            return Ok(receipt);
        }
        let row = tx
            .query_opt(
                "SELECT * FROM member_invitations WHERE id=$1 FOR UPDATE",
                &[&id.as_uuid()],
            )
            .map_err(|_| storage())?
            .ok_or(ApplicationError::UserNotFound)?;
        account_absent(&mut tx, &row.get::<_, String>("email"))?;
        if row.get::<_, Option<i64>>("accepted_at").is_some()
            || u64::try_from(row.get::<_, i64>("revision")).ok()
                != Some(c.request.expected_revision)
        {
            return Err(ApplicationError::ConcurrentModification);
        }
        let revision = i64::try_from(c.request.expected_revision)
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or(ApplicationError::ConcurrentModification)?;
        if c.request.operation_id.is_nil() {
            return Err(ApplicationError::InvalidInput(
                "invitation operation is required".into(),
            ));
        }
        let at = now(&mut tx)?;
        let until = at.checked_add(INVITATION_TTL_SECONDS).ok_or_else(storage)?;
        let audit = append_transaction(
            &mut tx,
            &actor.principal.email,
            "identity.invitation_resent",
            &format!(
                "invitation:{id}:revision:{revision}:operation:{}",
                c.request.operation_id
            ),
            seconds(at)?,
        )?;
        let seq = i64::try_from(audit.event.sequence).map_err(|_| storage())?;
        let row=tx.query_one("UPDATE member_invitations SET revision=$2,operation_id=$3,issued_at=$4,expires_at=$5,token_digest=$6,protected_token=$7,pending_operation=NULL,pending_digest=NULL,pending_material=NULL,delivery_state='pending',delivery_generation=0,lease_id=NULL,lease_until=NULL,provider_deadline=NULL,retry_at=NULL,cancelled_at=NULL,audit_sequence=$8 WHERE id=$1 RETURNING *",&[&id.as_uuid(),&revision,&c.request.operation_id,&at,&until,&&c.token_digest.as_bytes()[..],&c.protected_token,&seq]).map_err(|_|storage())?;
        let result = summary(&row, at)?;
        save_receipt(&mut tx, &actor, &result, command, seq)?;
        tx.commit().map_err(|_| storage())?;
        Ok(result)
    }
    pub(super) fn write_pending(
        &self,
        c: PrepareInvitation,
    ) -> Result<PendingInvitation, ApplicationError> {
        let mut client = self.lock()?;
        let mut tx = begin_audited(&mut client)?;
        let at = now(&mut tx)?;
        let row = live(&mut tx, c.id, c.revision, c.token_digest, at)?;
        account_absent(&mut tx, &row.get::<_, String>("email"))?;
        if let Some(p) = pending(&row)? {
            return Ok(p);
        }
        let audit = append_transaction(
            &mut tx,
            "invitation-recipient",
            "identity.invitation_prepared",
            &format!(
                "invitation:{}:revision:{}:operation:{}",
                c.id, c.revision, c.pending.operation_id
            ),
            seconds(at)?,
        )?;
        let seq = i64::try_from(audit.event.sequence).map_err(|_| storage())?;
        tx.execute("UPDATE member_invitations SET pending_operation=$2,pending_digest=$3,pending_material=$4,audit_sequence=$5 WHERE id=$1",&[&c.id.as_uuid(),&c.pending.operation_id,&&c.pending.request_digest.as_bytes()[..],&c.pending.protected_material,&seq]).map_err(|_|storage())?;
        tx.commit().map_err(|_| storage())?;
        Ok(c.pending)
    }
    pub(super) fn write_finish(
        &self,
        c: FinishInvitation,
    ) -> Result<InvitationReceipt, ApplicationError> {
        let mut client = self.lock()?;
        let mut tx = begin_audited(&mut client)?;
        if let Some(row)=tx.query_opt("SELECT request_digest,receipt FROM member_invitation_completions WHERE operation_id=$1",&[&c.operation_id]).map_err(|_|storage())? {
            let receipt=decode_receipt(row.get("receipt"))?;
            if digest(row.get("request_digest"))?!=c.request_digest || receipt.id!=c.id || receipt.revision!=c.revision{return Err(ApplicationError::ConcurrentModification)}
            return Ok(receipt)
        }
        let at = now(&mut tx)?;
        let row = live(&mut tx, c.id, c.revision, c.token_digest, at)?;
        let p = pending(&row)?.ok_or_else(invalid)?;
        if p.operation_id != c.pending_operation_id
            || c.user.email != row.get::<_, String>("email")
            || c.user.role.as_str() != row.get::<_, String>("role")
            || !c.user.active
            || c.user.revision != 0
            || c.user.auth_generation != 0
        {
            return Err(invalid());
        }
        account_absent(&mut tx, &c.user.email)?;
        let audit = append_transaction(
            &mut tx,
            &c.user.email,
            "identity.invitation_accepted",
            &format!(
                "invitation:{}:revision:{}:user:{}:operation:{}",
                c.id, c.revision, c.user.id, c.operation_id
            ),
            seconds(at)?,
        )?;
        let seq = i64::try_from(audit.event.sequence).map_err(|_| storage())?;
        crate::identity::postgres::insert_user(&mut tx, &c.user)?;
        let receipt = InvitationReceipt {
            id: c.id,
            revision: c.revision,
            operation_id: c.operation_id,
            user_id: c.user.id,
            email: c.user.email.clone(),
            role: c.user.role,
            accepted_at: seconds(at)?,
        };
        tx.execute("INSERT INTO member_invitation_completions(invitation_id,operation_id,token_digest,request_digest,user_id,receipt,audit_sequence) VALUES($1,$2,$3,$4,$5,$6,$7)",&[&c.id.as_uuid(),&c.operation_id,&&c.token_digest.as_bytes()[..],&&c.request_digest.as_bytes()[..],&c.user.id.as_uuid(),&encode_receipt(&receipt),&seq]).map_err(|_|storage())?;
        tx.execute("UPDATE member_invitations SET accepted_at=$2,protected_token=NULL,pending_operation=NULL,pending_digest=NULL,pending_material=NULL,lease_id=NULL,lease_until=NULL,retry_at=NULL,audit_sequence=$3 WHERE id=$1",&[&c.id.as_uuid(),&at,&seq]).map_err(|_|storage())?;
        tx.commit().map_err(|_| storage())?;
        Ok(receipt)
    }
}
fn replay(
    tx: &mut Transaction<'_>,
    actor: &InvitationActor,
    op: Uuid,
    command: &Value,
) -> Result<Option<InvitationSummary>, ApplicationError> {
    let row=tx.query_opt("SELECT actor_id,command,response FROM member_invitation_operations WHERE operation_id=$1",&[&op]).map_err(|_|storage())?;
    row.map(|r| {
        if r.get::<_, Uuid>("actor_id") != actor.principal.id.as_uuid()
            || r.get::<_, Value>("command") != *command
        {
            return Err(ApplicationError::ConcurrentModification);
        }
        decode_summary(r.get("response"))
    })
    .transpose()
}
fn save_receipt(
    tx: &mut Transaction<'_>,
    actor: &InvitationActor,
    s: &InvitationSummary,
    command: Value,
    seq: i64,
) -> Result<(), ApplicationError> {
    tx.execute("INSERT INTO member_invitation_operations(operation_id,actor_id,invitation_id,command,response,audit_sequence) VALUES($1,$2,$3,$4,$5,$6)",&[&s.operation_id,&actor.principal.id.as_uuid(),&s.id.as_uuid(),&command,&encode_summary(s),&seq]).map_err(|_|storage())?;
    Ok(())
}
fn live(
    tx: &mut Transaction<'_>,
    id: InvitationId,
    revision: u64,
    hash: domain::crypto::Sha256Digest,
    at: i64,
) -> Result<Row, ApplicationError> {
    let revision = i64::try_from(revision).map_err(|_| invalid())?;
    tx.query_opt("SELECT * FROM member_invitations WHERE id=$1 AND revision=$2 AND token_digest=$3 AND accepted_at IS NULL AND cancelled_at IS NULL AND expires_at>$4 FOR UPDATE",&[&id.as_uuid(),&revision,&&hash.as_bytes()[..],&at]).map_err(|_|storage())?.ok_or_else(invalid)
}
