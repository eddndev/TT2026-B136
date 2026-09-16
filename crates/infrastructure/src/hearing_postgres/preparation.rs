use super::{inconsistent, port, sources, storage};
use application::case_stages::StageSupportRef;
use application::documents::StageSupportReadLimits;
use application::participants::DirectoryStatus;
use application::{hearings::*, ApplicationError};
use domain::{
    case_administration::CaseAdministrativeStatus, cases::CaseId, crypto::DocumentHasher,
};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &HearingCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<HearingPreparation, ApplicationError> {
    command.result_revision()?;
    let used: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_hearing_revisions WHERE operation_id=$1)",
            &[&command.operation_id.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    if used {
        return Err(HearingError::OperationConflict.into());
    }
    let base = match storage::detail(tx, case, command.hearing_id, None, hasher) {
        Ok(value) => Some(value),
        Err(ApplicationError::Hearing(HearingError::NotFound)) => None,
        Err(error) => return Err(error),
    };
    match (&command.change, &base) {
        (HearingChange::Schedule { .. }, Some(_)) => {
            return Err(HearingError::RevisionConflict.into())
        }
        (HearingChange::Replace { .. } | HearingChange::Cancel { .. }, None) => {
            return Err(HearingError::NotFound.into())
        }
        _ => {}
    }
    if let Some(base) = &base {
        if base.snapshot.revision.get() != command.expected_revision() {
            return Err(HearingError::RevisionConflict.into());
        }
        if base.snapshot.status != HearingStatus::Scheduled {
            return Err(HearingError::AlreadyCancelled.into());
        }
    }
    let context = sources::context(tx, case, hasher)?;
    if context.administration.values().status() != CaseAdministrativeStatus::Active {
        return Err(ApplicationError::CaseClosed);
    }
    if matches!(command.change, HearingChange::Cancel { .. }) {
        let participants = base
            .as_ref()
            .ok_or_else(|| inconsistent("cancel has no base"))?
            .participants
            .clone();
        return Ok(HearingPreparation {
            base,
            context,
            participants,
            records: vec![],
        });
    }
    let (expected, values) = match &command.change {
        HearingChange::Schedule { context, values }
        | HearingChange::Replace {
            context, values, ..
        } => (*context, values),
        HearingChange::Cancel { .. } => unreachable!("cancellation returned above"),
    };
    let administration = context
        .administration
        .snapshot()
        .ok_or(HearingError::ContextRequired)?;
    let stage = context.stage.entry().ok_or(HearingError::ContextRequired)?;
    if administration.values.profile().is_none() {
        return Err(HearingError::ContextRequired.into());
    }
    if administration.revision != expected.case_revision
        || stage.stage_revision() != expected.stage_revision
    {
        return Err(HearingError::ContextConflict.into());
    }
    if base
        .as_ref()
        .is_some_and(|base| base.snapshot.values.kind() != values.kind())
    {
        return Err(HearingError::ImmutableKind.into());
    }
    if stage.stage() != values.kind().required_stage() {
        return Err(HearingError::StageIncompatible.into());
    }
    let mut participants = Vec::with_capacity(values.participants().len());
    for reference in values.participants() {
        let retained = base.as_ref().and_then(|base| {
            base.participants.iter().find(|old| {
                old.overview.id == reference.id() && old.overview.revision == reference.revision()
            })
        });
        let detail = if retained.is_some() {
            crate::participant_postgres::storage::exact(
                tx,
                case,
                reference.id(),
                reference.revision(),
                hasher,
            )
        } else {
            crate::participant_postgres::storage::current(tx, case, reference.id(), hasher)
        }
        .map_err(|error| match error {
            ApplicationError::ParticipantNotFound => HearingError::ParticipantChanged.into(),
            other => other,
        })?;
        let participant = sources::participant(detail)?;
        if let Some(retained) = retained {
            if retained != &participant {
                return Err(inconsistent("retained hearing participant source changed"));
            }
        } else if participant.overview.revision != reference.revision()
            || participant.overview.directory_status != DirectoryStatus::Active
        {
            return Err(HearingError::ParticipantChanged.into());
        }
        participants.push(participant);
    }
    let records = match values.conviction_basis() {
        Some(basis) => {
            let support = basis.support();
            vec![crate::case_stages::documents::load(
                tx,
                case,
                StageSupportRef::new(support.reference(), support.digest()),
                limits,
            )?]
        }
        None => vec![],
    };
    Ok(HearingPreparation {
        base,
        context,
        participants,
        records,
    })
}
