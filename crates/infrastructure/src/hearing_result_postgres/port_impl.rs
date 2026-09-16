use super::{authorization, port, preparation, storage, PostgresHearingResultStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{documents::StageSupportReadLimits, hearing_results::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, hearings::HearingId, identity::UserId};

impl HearingResultStore for PostgresHearingResultStore {
    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        hearing: HearingId,
        query: HearingResultQuery,
        at: OffsetDateTime,
    ) -> Result<HearingResultPage, ApplicationError> {
        self.list_page(actor, case, hearing, query, at)
    }
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        hearing: HearingId,
        id: HearingResultId,
        revision: Option<HearingResultRevision>,
        at: OffsetDateTime,
    ) -> Result<HearingResultDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        let detail = storage::detail(&mut tx, case, hearing, id, revision, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            "hearing_result.read",
            &format!(
                "case:{case}:hearing:{hearing}:result:{id}:revision:{}:sha256:{}",
                detail.snapshot.revision.get(),
                detail.snapshot.values_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        hearing: HearingId,
        id: HearingResultId,
        query: HearingResultHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<HearingResultHistoryPage, ApplicationError> {
        self.history_page(actor, case, hearing, id, query, at)
    }
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        command: &HearingResultCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<HearingResultPreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorization::authorize(&mut tx, actor, case, true)?;
        let result = preparation::load(&mut tx, case, command, limits, self.hasher.as_ref())?;
        tx.rollback().map_err(port)?;
        Ok(result)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedHearingResultChange,
    ) -> Result<HearingResultDetail, ApplicationError> {
        self.commit_change(actor, case, prepared)
    }
}
