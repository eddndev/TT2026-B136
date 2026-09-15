//! Transactional stages whose supports remain bound to exact encrypted snapshots.

mod authorization;
pub(crate) mod decode;
mod documents;
mod encode;
pub(crate) mod query;

use crate::audit_postgres::{append_transaction, begin_audited};
use application::case_stages::*;
use application::cases::CaseActorSnapshot;
use application::ApplicationError;
use authorization::{authorize, check_head, context};
use domain::cases::CaseId;
use domain::crypto::DocumentHasher;
use domain::identity::UserId;
use postgres::Client;
use std::sync::{Arc, Mutex, MutexGuard};
use time::OffsetDateTime;

pub struct PostgresCaseStageStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
}
impl PostgresCaseStageStore {
    pub fn connect(
        url: &str,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::connect(url)?),
            hasher,
        })
    }
    pub fn open(
        url: &str,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::open(url)?),
            hasher,
        })
    }
    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client
            .lock()
            .map_err(|_| ApplicationError::Port("stage database lock poisoned".into()))
    }
}
impl CaseStageStore for PostgresCaseStageStore {
    fn get(
        &self,
        actor: UserId,
        case_id: CaseId,
        at: OffsetDateTime,
    ) -> Result<CaseStageDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case_id, CaseStageAction::Read)?;
        let current = query::current(&mut tx, case_id, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            CaseStageAction::Read.as_str(),
            &format!(
                "case:{case_id}:stage:{}",
                current.revision().map_or(0, CaseStageRevision::get)
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(CaseStageDetail { case_id, current })
    }
    fn history(
        &self,
        actor: UserId,
        case_id: CaseId,
        query: CaseStageQuery,
        at: OffsetDateTime,
    ) -> Result<CaseStagePage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case_id, CaseStageAction::History)?;
        let page = query::history(&mut tx, case_id, &query, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            CaseStageAction::History.as_str(),
            &format!("case:{case_id}:stages"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(page)
    }
    fn prepare(
        &self,
        actor: UserId,
        case_id: CaseId,
        expected: CaseStageExpectation,
        change: &CaseStageChange,
        limits: &StageSupportReadLimits,
    ) -> Result<CaseStagePreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case_id, action(change))?;
        documents::require_scope(&mut tx, case_id, &change.supports())?;
        context(&mut tx, case_id)?;
        let current = query::current(&mut tx, case_id, self.hasher.as_ref())?;
        check_head(&current, expected, change)?;
        let records = change
            .supports()
            .into_iter()
            .map(|support| documents::load(&mut tx, case_id, support, limits))
            .collect::<Result<Vec<_>, _>>()?;
        tx.rollback().map_err(port)?;
        Ok(CaseStagePreparation { current, records })
    }
    fn commit(
        &self,
        actor: UserId,
        case_id: CaseId,
        expected: CaseStageExpectation,
        prepared: PreparedCaseStageChange,
        at: OffsetDateTime,
    ) -> Result<CaseStageDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let action = action(prepared.change());
        let principal = authorize(&mut tx, actor, case_id, action)?;
        documents::require_scope(&mut tx, case_id, &prepared.change().supports())?;
        let (administration_revision, administration_digest) = context(&mut tx, case_id)?;
        let current = query::current(&mut tx, case_id, self.hasher.as_ref())?;
        let stage_revision = check_head(&current, expected, prepared.change())?;
        prepared.change().validate_recording_at(at)?;
        let limits = StageSupportReadLimits::standard();
        for support in prepared.supports() {
            let record = support.record();
            let reference = StageSupportRef::new(
                application::documents::DocumentVersionRef {
                    id: record.id,
                    version: record.version,
                },
                record.digest,
            );
            let sealed:bool=tx.query_one("SELECT evidence IS NOT NULL FROM documents WHERE id=$1 AND case_id=$2 AND version=$3",&[&record.id.as_uuid(),&case_id.as_uuid(),&i64::from(record.version.get())]).map_err(port)?.get(0);
            if sealed != record.evidence.is_some() {
                return Err(ApplicationError::StageSupportChanged);
            }
            let current_record = documents::load(&mut tx, case_id, reference, &limits).map_err(
                |error| match error {
                    ApplicationError::StageSupportDigestMismatch => {
                        ApplicationError::StageSupportChanged
                    }
                    other => other,
                },
            )?;
            if current_record != *record {
                return Err(ApplicationError::StageSupportChanged);
            }
        }
        let snapshot = CaseStageSnapshot {
            case_id,
            stage_revision,
            from_stage: current.stage(),
            values: prepared.change().clone(),
            values_digest: case_stage_digest(self.hasher.as_ref(), prepared.change()),
            administration_revision,
            administration_digest,
            supports: prepared
                .supports()
                .iter()
                .map(ValidatedStageSupport::snapshot)
                .collect(),
            recorded_at: at.to_offset(time::UtcOffset::UTC),
            recorded_by: CaseActorSnapshot {
                id: principal.id,
                email: principal.email.clone(),
            },
        };
        encode::insert(&mut tx, &snapshot)?;
        append_transaction(
            &mut tx,
            &principal.email,
            action.as_str(),
            &format!(
                "case:{case_id}:stage:{}:administration:{}:sha256:{}",
                stage_revision.get(),
                administration_revision.get(),
                snapshot.values_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(CaseStageDetail {
            case_id,
            current: CurrentCaseStage::Registered(Box::new(CaseStageEntry::Changed(Box::new(
                snapshot,
            )))),
        })
    }
}
fn action(change: &CaseStageChange) -> CaseStageAction {
    match change {
        CaseStageChange::Adopt(_) => CaseStageAction::Adopt,
        CaseStageChange::Transition(_) => CaseStageAction::Transition,
    }
}
pub(super) fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("stage database: {error}"))
}
pub(super) fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::StoredCaseStageInconsistent(error.to_string())
}
