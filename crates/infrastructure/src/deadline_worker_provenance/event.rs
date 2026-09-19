use super::{inconsistent, port, source_error};
use application::{
    deadline_profiles::{DeadlineProfileId, DeadlineProfileRevision, DeadlineProfileScope},
    deadline_reevaluation::{DependencyFamily, SourceEventReference},
    hearing_results::{HearingResultId, HearingResultRevision},
    judicial_calendars::{JudicialCalendarId, JudicialCalendarRevision},
    procedural_facts::{FactRevision, FactTarget, NotificationId, ResolutionId},
    ApplicationError,
};
use domain::{crypto::DocumentHasher, hearings::HearingId, typed_participants::Uuid};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    sequence: i64,
    hasher: &dyn DocumentHasher,
) -> Result<SourceEventReference, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT sequence,source_kind,source_id,revision,case_id,hearing_id,operation_id
        FROM deadline_source_events WHERE sequence=$1 AND octet_length(source_kind)<=14",
            &[&sequence],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("job source event is absent or unbounded"))?;
    let event = crate::deadline_dispatch_postgres::decode_source_event(&row)?;
    if i64::try_from(event.sequence).map_err(inconsistent)? != sequence {
        return Err(inconsistent("job source event sequence differs"));
    }
    let actual = exact(tx, event, hasher).map_err(source_error)?;
    if actual != event {
        return Err(inconsistent(
            "job source event differs from its exact historical revision",
        ));
    }
    Ok(event)
}

/// Reconstruct every revision-owned field from the existing verified reader;
/// only sequence belongs exclusively to the durable event ledger.
fn exact(
    tx: &mut Transaction<'_>,
    event: SourceEventReference,
    hasher: &dyn DocumentHasher,
) -> Result<SourceEventReference, ApplicationError> {
    let (family, source_id, revision, case_id, hearing_id, operation_id) = match event.family {
        DependencyFamily::Calendar => {
            let value = crate::judicial_calendar_postgres::storage::detail(
                tx,
                JudicialCalendarId::from_uuid(event.source_id),
                Some(JudicialCalendarRevision::new(event.revision).map_err(inconsistent)?),
                hasher,
            )?;
            (
                DependencyFamily::Calendar,
                value.id.as_uuid(),
                value.revision.get(),
                None,
                None,
                value.receipt.operation_id.as_uuid(),
            )
        }
        DependencyFamily::Profile => {
            let value = crate::deadline_profile_postgres::storage::detail(
                tx,
                DeadlineProfileId::from_uuid(event.source_id),
                Some(DeadlineProfileRevision::new(event.revision).map_err(inconsistent)?),
                hasher,
            )?;
            let scope = match value.definition.scope() {
                DeadlineProfileScope::Global(_) => None,
                DeadlineProfileScope::Case(case) => Some(*case),
            };
            (
                DependencyFamily::Profile,
                value.id.as_uuid(),
                value.revision.get(),
                scope,
                None,
                value.receipt.operation_id.as_uuid(),
            )
        }
        DependencyFamily::HearingResult => {
            let case = event
                .case_id
                .ok_or_else(|| inconsistent("result event case is absent"))?;
            let hearing = event
                .hearing_id
                .ok_or_else(|| inconsistent("result event hearing is absent"))?;
            let value = crate::hearing_result_postgres::storage::detail(
                tx,
                case,
                HearingId::from_uuid(hearing),
                HearingResultId::from_uuid(event.source_id),
                Some(HearingResultRevision::new(event.revision).map_err(inconsistent)?),
                hasher,
            )?;
            let value = value.snapshot;
            (
                DependencyFamily::HearingResult,
                value.id.as_uuid(),
                value.revision.get(),
                Some(value.case_id),
                Some(value.hearing_id.as_uuid()),
                value.receipt.operation_id.as_uuid(),
            )
        }
        DependencyFamily::Resolution | DependencyFamily::Notification => {
            let case = event
                .case_id
                .ok_or_else(|| inconsistent("fact event case is absent"))?;
            let target = match event.family {
                DependencyFamily::Resolution => {
                    FactTarget::Resolution(ResolutionId::from_uuid(event.source_id))
                }
                DependencyFamily::Notification => {
                    let parent: Uuid = tx
                        .query_opt(
                            "SELECT parent_resolution_id FROM case_procedural_facts
                        WHERE family='notification' AND id=$1 AND case_id=$2",
                            &[&event.source_id, &case.as_uuid()],
                        )
                        .map_err(port)?
                        .ok_or_else(|| inconsistent("notification event root is absent"))?
                        .try_get(0)
                        .map_err(inconsistent)?;
                    FactTarget::Notification {
                        id: NotificationId::from_uuid(event.source_id),
                        resolution_id: ResolutionId::from_uuid(parent),
                    }
                }
                _ => return Err(inconsistent("unsupported fact event family")),
            };
            let value = crate::procedural_fact_postgres::storage::detail(
                tx,
                case,
                target,
                Some(FactRevision::new(event.revision).map_err(inconsistent)?),
                hasher,
            )?;
            let metadata = value.snapshot.metadata();
            let (family, id) = match value.snapshot.target() {
                FactTarget::Resolution(id) => (DependencyFamily::Resolution, id.as_uuid()),
                FactTarget::Notification { id, .. } => {
                    (DependencyFamily::Notification, id.as_uuid())
                }
            };
            (
                family,
                id,
                metadata.revision.get(),
                Some(value.snapshot.case_id()),
                None,
                metadata.receipt.operation_id.as_uuid(),
            )
        }
    };
    Ok(SourceEventReference {
        sequence: event.sequence,
        family,
        source_id,
        revision,
        case_id,
        hearing_id,
        operation_id,
    })
}
