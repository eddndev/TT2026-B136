use super::*;
use application::{
    participants::{DirectoryStatus, ParticipantStore},
    precautionary_hearings::{PrecautionaryHearingChange, PrecautionaryHearingCommand},
};
use domain::{
    hearings::{HearingModality, HearingParticipantRef, HearingTime, HearingVenue},
    precautionary_hearings::*,
};
use infrastructure::PostgresParticipantStore;

#[test]
fn reopened_reads_retain_original_g_h_a_sources_after_directory_and_hearing_changes() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = crate::measure_fixture::setup(&mut db);
    let participant =
        HearingParticipantRef::new(seed.supervisor.id(), seed.supervisor.revision_number());
    let old = crate::measure_fixture::values(&seed.subject);
    let values = MeasureValues::new(MeasureValuesInput {
        subject: old.subject(),
        kind: old.kind(),
        conditions: old.conditions().clone(),
        validity: old.validity().clone(),
        supervision: MeasureSupervision::Known {
            participant,
            statement: note("Exact original supervisor"),
        },
    });
    seed.command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Impose(MeasureProposal {
            id: MeasureId::new(),
            values,
        }),
    ]))
    .unwrap();
    let hearing = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        PrecautionaryHearingCommand {
            operation_id: PrecautionaryHearingOperationId::new(),
            hearing_id: PrecautionaryHearingId::new(),
            change: PrecautionaryHearingChange::Schedule {
                context: seed.command.context,
                values: PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
                    purpose: PrecautionaryHearingPurpose::Imposition,
                    scheduled_at: HearingTime::new(
                        (db.at + time::Duration::days(2))
                            .replace_nanosecond(0)
                            .unwrap(),
                    )
                    .unwrap(),
                    modality: HearingModality::InPerson,
                    venue: HearingVenue::new("Original hearing room").unwrap(),
                    note: None,
                    participants: vec![participant],
                    scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
                        note("Declared appointment"),
                        seed.command.values.support(),
                        note("Page 1"),
                    ),
                    review_targets: vec![],
                })
                .unwrap(),
            },
        },
    );
    let selected = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        crate::hearing_fixture::replacement(&hearing),
    );
    seed.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: selected.capture.review.command.hearing_id,
        revision: selected.capture.review.result_revision,
        capture_digest: selected.capture.capture_digest,
    });
    let judicial = crate::measure_fixture::persist(&db, seed.actor.clone(), seed.command.clone());
    let previous = &judicial.group.measures[0];
    let first = persist(
        &db,
        seed.actor.clone(),
        correction(
            reference(previous),
            seed.command.context,
            &previous.result.values,
            "Corrected text retaining original anchored sources",
        ),
    );
    let marked = persist(
        &db,
        seed.actor.clone(),
        mark(corrected_reference(&first.capture), seed.command.context),
    );
    let participants =
        PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    participants
        .change_status(
            db.owner,
            db.case,
            seed.supervisor.id(),
            seed.supervisor.revision_number(),
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    let cancelled = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        crate::hearing_fixture::cancellation(&selected),
    );
    assert_eq!(cancelled.history.captures.len(), 3);
    let service = reads(&db, seed.actor);
    for original in [&first, &marked] {
        let actual = service
            .get_operation("session", db.case, original.origin.operation_id)
            .unwrap();
        same_operation(&actual, original);
        assert_eq!(
            actual.capture.review.result.sources.supervisor.as_ref(),
            Some(&seed.supervisor)
        );
        assert_eq!(actual.capture.review.result.sources.subject, seed.subject);
        assert_eq!(
            actual.capture.review.support,
            judicial.group.decision.support
        );
        assert_eq!(
            actual.record_history.records.judicial.groups[0].capture,
            judicial.group
        );
        assert_eq!(
            actual.record_history.records.judicial.groups[0]
                .capture
                .review
                .material
                .anchor,
            Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
                selected.capture.clone()
            )))
        );
    }
    let page = service
        .list(
            "session",
            db.case,
            MeasureAdministrativeReadQuery::default(),
        )
        .unwrap();
    assert_eq!(page.items.len(), 2);
    for original in [&first, &marked] {
        let actual = page
            .items
            .iter()
            .find(|a| a.origin == original.origin)
            .unwrap();
        same_operation(actual, original);
    }
    assert_eq!(
        marked.record_history.records.administrative[0].capture,
        first.capture
    );
}
