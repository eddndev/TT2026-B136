//! One audited transaction resolves exact deadline inputs and their observed heads.
use application::{
    deadline_inputs::*, identity::Principal, procedural_facts::FactTarget, ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::DocumentHasher,
    deadline_triggers::TriggerSourceRef,
    identity::{Permission, Role, UserId},
    procedural_facts::FactDeclaration,
};
use postgres::{Client, Transaction};
use std::sync::{Arc, Mutex};

pub struct PostgresDeadlineInputStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresDeadlineInputStore {
    pub fn open(
        url: &str,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::open(url)?),
            hasher,
            clock,
        })
    }
}
impl DeadlineInputStore for PostgresDeadlineInputStore {
    fn load(
        &self,
        actor: UserId,
        request: &DeadlineInputRequest,
    ) -> Result<DeadlineInputMaterial, ApplicationError> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| ApplicationError::Port("deadline input database lock poisoned".into()))?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let case_id = request.trigger.case_id;
        let principal = authorize(&mut tx, actor, case_id)?;
        let administration =
            crate::cases::storage::detail(&mut tx, case_id, self.hasher.as_ref())?.administration;
        let (source, source_head) = match request.trigger.source {
            FactDeclaration::Unknown(_) => (None, None),
            FactDeclaration::Known(reference) => (
                Some(source(
                    &mut tx,
                    case_id,
                    reference,
                    true,
                    self.hasher.as_ref(),
                )?),
                Some(source(
                    &mut tx,
                    case_id,
                    reference,
                    false,
                    self.hasher.as_ref(),
                )?),
            ),
        };
        let (calendar, calendar_head) = match request.calendar {
            None => (None, None),
            Some(selected) => (
                Some(crate::judicial_calendar_postgres::storage::detail(
                    &mut tx,
                    selected.id,
                    Some(selected.revision),
                    self.hasher.as_ref(),
                )?),
                Some(crate::judicial_calendar_postgres::storage::detail(
                    &mut tx,
                    selected.id,
                    None,
                    self.hasher.as_ref(),
                )?),
            ),
        };
        let material = DeadlineInputMaterial {
            case_id,
            administration,
            source,
            source_head,
            calendar,
            calendar_head,
        };
        check_deadline_inputs(self.hasher.as_ref(), request, &material)?;
        crate::audit_postgres::append_transaction(
            &mut tx,
            &principal.email,
            "deadline.inputs_read",
            &format!("case:{case_id}:deadline_inputs"),
            self.clock.now(),
        )?;
        tx.commit().map_err(port)?;
        Ok(material)
    }
}
fn authorize(
    tx: &mut Transaction<'_>,
    actor: UserId,
    case_id: CaseId,
) -> Result<Principal, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, actor)?;
    if !principal.role.allows(Permission::ReadDeadlineInputs) {
        return Err(ApplicationError::PermissionDenied);
    }
    let visible = if principal.role == Role::Owner {
        tx.query_opt("SELECT id FROM cases WHERE id=$1", &[&case_id.as_uuid()])
    } else {
        tx.query_opt(
            "SELECT case_id FROM case_memberships WHERE case_id=$1 AND user_id=$2 FOR SHARE",
            &[&case_id.as_uuid(), &principal.id.as_uuid()],
        )
    }
    .map_err(port)?;
    visible.ok_or(ApplicationError::CaseNotFound)?;
    Ok(principal)
}
fn source(
    tx: &mut Transaction<'_>,
    case_id: CaseId,
    reference: TriggerSourceRef,
    exact: bool,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineSourceDetail, ApplicationError> {
    let (target, revision) = match reference {
        TriggerSourceRef::Resolution(r) => (FactTarget::Resolution(r.id), r.revision),
        TriggerSourceRef::Notification {
            id,
            revision,
            resolution,
        } => (
            FactTarget::Notification {
                id,
                resolution_id: resolution.id,
            },
            revision,
        ),
        TriggerSourceRef::HearingResult(r) => {
            return Ok(DeadlineSourceDetail::HearingResult(Box::new(
                crate::hearing_result_postgres::storage::detail(
                    tx,
                    case_id,
                    r.hearing_id,
                    r.result_id,
                    exact.then_some(r.revision),
                    hasher,
                )?,
            )))
        }
    };
    Ok(DeadlineSourceDetail::Fact(Box::new(
        crate::procedural_fact_postgres::storage::detail(
            tx,
            case_id,
            target,
            exact.then_some(revision),
            hasher,
        )?,
    )))
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("deadline input database: {error}"))
}
