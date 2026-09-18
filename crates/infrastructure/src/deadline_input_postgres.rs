//! One audited transaction resolves exact deadline inputs and their observed heads.
use application::{
    deadline_inputs::*, identity::Principal, procedural_facts::FactTarget, ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::DocumentHasher,
    deadline_triggers::{TriggerSelection, TriggerSourceRef},
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
        let material = load_material(
            &mut tx,
            &request.trigger,
            request.calendar,
            self.hasher.as_ref(),
        )?;
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
/// Resolve material using the caller's transaction and prior case authorization.
/// The caller must check extraction and append its audit before committing.
pub(crate) fn load_material(
    tx: &mut Transaction<'_>,
    selection: &TriggerSelection,
    calendar: Option<DeadlineCalendarRef>,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineInputMaterial, ApplicationError> {
    let case_id = selection.case_id;
    let administration = crate::cases::storage::detail(tx, case_id, hasher)?.administration;
    let (source, source_head) = match selection.source {
        FactDeclaration::Unknown(_) => (None, None),
        FactDeclaration::Known(reference) => (
            Some(source(tx, case_id, reference, true, hasher)?),
            Some(source(tx, case_id, reference, false, hasher)?),
        ),
    };
    let (calendar, calendar_head) = match calendar {
        None => (None, None),
        Some(selected) => (
            Some(crate::judicial_calendar_postgres::storage::detail(
                tx,
                selected.id,
                Some(selected.revision),
                hasher,
            )?),
            Some(crate::judicial_calendar_postgres::storage::detail(
                tx,
                selected.id,
                None,
                hasher,
            )?),
        ),
    };
    Ok(DeadlineInputMaterial {
        case_id,
        administration,
        source,
        source_head,
        calendar,
        calendar_head,
    })
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
pub(crate) fn source(
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
