use super::*;
use domain::identity::Role;

impl PostgresCaseReportStore {
    pub(super) fn request_report(
        &self,
        actor: &Principal,
        scope: CaseReportScope,
        command: CaseReportCommand,
        wanted: Sha256Digest,
        at: OffsetDateTime,
    ) -> Result<CaseReportDetail, ApplicationError> {
        if !matches!(actor.role, Role::Owner | Role::Litigator) {
            return Err(ApplicationError::PermissionDenied);
        }
        let calculated = case_report_request_digest(self.hasher.as_ref(), actor, scope, &command)?;
        if calculated != wanted {
            return Err(ApplicationError::InvalidInput(
                "report request digest differs".into(),
            ));
        }
        self.tx(|tx| {
            let stamp = access::actor(tx, actor)?;
            let now = self.observed(at)?;
            if let Some(row) = tx.query_opt("SELECT id FROM case_report_jobs WHERE requester_id=$1 AND operation_id=$2",
                &[&actor.id.as_uuid(), &command.operation_id.as_uuid()]).map_err(port)? {
                let saved = storage::get(tx, CaseReportId::from_uuid(row.get(0)), self.hasher.as_ref())?;
                access::original(tx, &saved)?;
                if saved.detail.command != command || saved.detail.request_digest != wanted || saved.detail.scope != scope {
                    return Err(CaseReportError::OperationConflict.into());
                }
                return Ok(saved.detail);
            }
            access::filter_member(tx, actor, command.filters.assigned_litigator)?;
            let id = CaseReportId::new();
            tx.execute("INSERT INTO case_report_jobs(id,requester_id,principal,account_revision,auth_generation,scope,operation_id,command,request_digest,requested_at,updated_at,state)
                VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$10,'queued')", &[&id.as_uuid(), &actor.id.as_uuid(),
                &serde_json::to_value(actor).map_err(inconsistent)?, &(stamp.account_revision as i64), &(stamp.auth_generation as i64),
                &scope_name(scope), &command.operation_id.as_uuid(), &codec::command(&command)?, &&wanted.as_bytes()[..], &timestamp(now)?]).map_err(port)?;
            let saved = storage::get(tx, id, self.hasher.as_ref())?;
            audit(tx, &saved.detail, "case_report.request", now)?;
            Ok(saved.detail)
        })
    }
}
