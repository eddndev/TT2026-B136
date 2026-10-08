use super::*;
use application::typed_participants::{
    IdentityDifferentDecision, IdentityReviewSubmission, ParticipantPreparationRequest,
    ParticipantSubmission, SubjectDraftSelection, TypedParticipantWorkflow,
};
use domain::hearings::HearingParticipantRef;
use domain::typed_participants::{
    Declared, ParticipantReason, ParticipantText, RepresentedName, SubjectRevisionRef,
    SubjectValues,
};

pub(super) fn setup_joint(
    db: &mut Fixture,
) -> (
    Seed,
    MeasureDecisionStoredOperation,
    SubjectSnapshot,
    MeasureAdministrativeCommand,
) {
    let mut seed = crate::measure_fixture::setup(db);
    let base = crate::measure_fixture::values(&seed.subject);
    let values = MeasureValues::new(MeasureValuesInput {
        subject: base.subject(),
        kind: base.kind(),
        conditions: base.conditions().clone(),
        validity: base.validity().clone(),
        supervision: MeasureSupervision::Known {
            participant: HearingParticipantRef::new(
                seed.supervisor.id(),
                seed.supervisor.revision_number(),
            ),
            statement: note("Retain the exact declared supervisor and its bound subject"),
        },
    });
    seed.command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Impose(MeasureProposal {
            id: MeasureId::new(),
            values,
        }),
    ]))
    .unwrap();
    let judicial = crate::measure_fixture::persist(db, seed.actor.clone(), seed.command.clone());
    let subject = new_subject(db, &seed);
    let command = replacement_command(
        reference(&judicial.group.measures[0]),
        seed.command.context,
        &subject,
    );
    (seed, judicial, subject, command)
}

fn new_subject(db: &Fixture, seed: &Seed) -> SubjectSnapshot {
    let workflow = crate::typed_participant_service_support::service(db, FormatCheck(None));
    let mut proposal = crate::typed_participant_service_support::proposal(&seed.record);
    proposal.subject = SubjectDraftSelection::Create(SubjectValues::natural_person(
        RepresentedName::Known(ParticipantText::new("Beatriz Replacement").unwrap()),
        Declared::Unknown(
            ParticipantReason::new("Not declared for the selected identity").unwrap(),
        ),
        seed.subject.values.identity_support().clone(),
    ));
    let review = workflow
        .review_participant("session", db.case, proposal)
        .unwrap();
    let different = review
        .candidates
        .iter()
        .map(|candidate| IdentityDifferentDecision {
            candidate: candidate.reference.clone(),
            reason: ParticipantText::new("The document names two different represented people")
                .unwrap(),
            support: seed.subject.values.identity_support().clone(),
        })
        .collect();
    let reviewed = ParticipantPreparationRequest {
        proposal: review.proposal,
        review: IdentityReviewSubmission {
            directory_stamp: review.directory_stamp,
            different,
            selection_reason: ParticipantReason::new(
                "Select the other person named in the support",
            )
            .unwrap(),
        },
        certificate: None,
    };
    workflow
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: reviewed,
                signature: None,
            },
        )
        .unwrap()
        .bound_subject
        .unwrap()
}

pub(super) fn replacement_command(
    target: PrecautionaryMeasureRef,
    context: PrecautionaryContextExpectation,
    subject: &SubjectSnapshot,
) -> MeasureAdministrativeCommand {
    MeasureAdministrativeCommand {
        operation_id: MeasureCorrectionOperationId::new(),
        target,
        context,
        reason: note("The subject was entered incorrectly; retain the old declaration"),
        action: MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
            replacement_id: MeasureId::from_uuid(uuid::Uuid::nil()),
            subject: SubjectRevisionRef {
                id: subject.id,
                revision: subject.revision,
                values_digest: subject.values_digest,
            },
        },
    }
}

pub(super) fn replacement_row(
    operation: &MeasureAdministrativeStoredOperation,
) -> &MeasureAdministrativeRecordCapture {
    let expected = operation.capture.review.replacement.as_ref().unwrap();
    operation
        .capture
        .records
        .iter()
        .find(|row| row.result.id == expected.id)
        .unwrap()
}

pub(super) fn row_reference(row: &MeasureAdministrativeRecordCapture) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(row.result.id, row.result.revision, row.capture_digest)
}

pub(super) fn assert_joint(
    stored: &MeasureAdministrativeStoredOperation,
    subject: &SubjectSnapshot,
    old: &MeasureValues,
) {
    let capture = &stored.capture;
    let review = &capture.review;
    let new = review.replacement.as_ref().unwrap();
    let old_row = capture
        .records
        .iter()
        .find(|row| row.result.id == review.result.id)
        .unwrap();
    let new_row = replacement_row(stored);
    assert_eq!(capture.records.len(), 2);
    assert_eq!(old_row.result, review.result);
    assert_eq!(new_row.result, *new);
    assert_eq!(
        review.result.validity,
        MeasureCaptureValidity::EnteredInError
    );
    assert_eq!(new.validity, MeasureCaptureValidity::Valid);
    assert_eq!(new.revision.get(), 1);
    assert_eq!(new.previous, review.command.target);
    assert_ne!(new.id, review.result.id);
    assert_eq!(
        new.record_root,
        MeasureRecordRoot::Administrative {
            operation_id: review.command.operation_id,
            measure_id: new.id,
        }
    );
    assert_eq!(
        new.values.subject(),
        SubjectRevisionRef {
            id: subject.id,
            revision: subject.revision,
            values_digest: subject.values_digest,
        }
    );
    assert_eq!(new.sources.subject, *subject);
    assert_eq!(new.values.kind(), old.kind());
    assert_eq!(new.values.conditions(), old.conditions());
    assert_eq!(new.values.validity(), old.validity());
    assert_eq!(new.values.supervision(), old.supervision());
    assert_eq!(new.last_judicial, review.result.last_judicial);
    assert_eq!(new.judicial_origin, review.result.judicial_origin);
    assert_eq!(new.last_action, review.result.last_action);
    assert_eq!(new.sources.supervisor, review.result.sources.supervisor);
    assert_eq!(
        capture.replacement_link,
        Some(MeasureAdministrativeReplacementLink {
            entered_in_error: row_reference(old_row),
            replacement: row_reference(new_row),
        })
    );
    for row in &capture.records {
        assert_eq!(row.operation_id, review.command.operation_id);
        assert_eq!(row.support, review.support);
        assert_eq!(row.recorded_at, capture.recorded_at);
    }
}

pub(super) fn assert_heads(
    db: &Fixture,
    actor: &Principal,
    stored: &MeasureAdministrativeStoredOperation,
) {
    let storage = crate::measure_fixture::store(db);
    for row in &stored.capture.records {
        let detail =
            MeasureRecordReadStore::get(storage.as_ref(), actor, db.case, row.result.id).unwrap();
        assert_eq!(detail.reference, row_reference(row));
        let OwnedMeasureRecord::Administrative { owner, capture } = detail.record else {
            panic!("replacement heads must retain their administrative owner")
        };
        assert_eq!(owner.operation_id, stored.origin.operation_id);
        assert_eq!(owner.capture_digest, stored.capture.capture_digest);
        assert_eq!(*capture, *row);
        let owner = detail
            .record_history
            .records
            .administrative
            .iter()
            .find(|entry| entry.origin.operation_id == stored.origin.operation_id)
            .unwrap();
        assert_eq!(owner.capture, stored.capture);
    }
}
