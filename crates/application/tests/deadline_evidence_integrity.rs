#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;

use application::{
    case_stages::{StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot},
    cases::CurrentCaseAdministration,
    deadline_inputs::DeadlineSourceDetail,
    deadline_profiles::*,
    deadlines::*,
    hearing_results::*,
    hearings::HearingParticipantSnapshot,
    procedural_facts::fact_receipt_matches,
    typed_participants::ParticipantOverview,
};
use deadline_support::{evaluation::inputs, *};
use domain::{
    cases::CaseMetadata,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    deadline_triggers::{TriggerField, TriggerRequirement},
    participants::{DirectoryStatus, ParticipantId, ParticipantRevision},
};
use uuid::Uuid;

#[test]
fn a_valid_fact_administration_substitution_invalidates_the_deadline_receipt() {
    let (command, preparation) = fixture();
    let mut stored = detail(&prepare(command, preparation).unwrap());
    deadline_receipt_matches(inputs::hasher().as_ref(), &stored).unwrap();
    for source in [
        &mut stored.calculation.material.source,
        &mut stored.calculation.material.source_head,
    ] {
        let Some(DeadlineSourceDetail::Fact(source)) = source else {
            unreachable!()
        };
        inputs::metadata_mut(source).recorded_administration = CurrentCaseAdministration::Unrevised(
            CaseMetadata::new("Different historical title", "Different reference").unwrap(),
        );
        fact_receipt_matches(inputs::hasher().as_ref(), source).unwrap();
    }
    assert!(deadline_receipt_matches(inputs::hasher().as_ref(), &stored).is_err());
}

#[test]
fn a_nonempty_hearing_attendee_name_substitution_invalidates_the_deadline_receipt() {
    let mut stored = hearing_deadline();
    for_hearing_sources(&mut stored, |source| {
        source.attendees[0].participant.overview.display_name =
            "Substituted historical name".into();
    });
    assert!(deadline_receipt_matches(inputs::hasher().as_ref(), &stored).is_err());
}

#[test]
fn a_hearing_support_name_substitution_invalidates_the_deadline_receipt() {
    let mut stored = hearing_deadline();
    for_hearing_sources(&mut stored, |source| {
        source.support.as_mut().unwrap().name = "different.pdf".into();
    });
    assert!(deadline_receipt_matches(inputs::hasher().as_ref(), &stored).is_err());
}

fn for_hearing_sources(stored: &mut DeadlineDetail, mutate: impl Fn(&mut HearingResultDetail)) {
    for source in [
        &mut stored.calculation.material.source,
        &mut stored.calculation.material.source_head,
    ] {
        let Some(DeadlineSourceDetail::HearingResult(source)) = source else {
            unreachable!()
        };
        mutate(source);
        hearing_result_receipt_matches(inputs::hasher().as_ref(), source).unwrap();
    }
}

fn hearing_deadline() -> DeadlineDetail {
    let mut source = inputs::hearing(1, false, "2026-01-06", &[]);
    let participant = ParticipantId::from_uuid(Uuid::from_u128(801));
    let revision = ParticipantRevision::new(1).unwrap();
    let support = StageSupportSnapshot {
        reference: DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(802)),
            version: DocumentVersion::initial(),
        },
        digest: Sha256Digest::from_array([18; 32]),
        name: "historical.pdf".into(),
        format: StageDocumentFormat::Pdf,
        policy: StageFormatPolicy::PdfDocxV1,
    };
    let v = &source.snapshot.values;
    source.snapshot.values = HearingResultValues::new(HearingResultValuesInput {
        occurrence: v.occurrence(),
        extent: v.extent(),
        event_time: v.event_time(),
        summary: v.summary().clone(),
        attendees: vec![HearingResultAttendee::new(
            participant,
            revision,
            HearingResultCapacity::new("Declared attendee").unwrap(),
            None,
        )],
        agreements: v.agreements().to_vec(),
        provenance: HearingResultProvenance::new(
            HearingResultProvenanceKind::OperatorNote,
            None,
            Some(HearingResultSupportRef::new(
                support.reference,
                support.digest,
            )),
        )
        .unwrap(),
    })
    .unwrap();
    source.attendees.push(HearingResultAttendeeSnapshot {
        participant: HearingParticipantSnapshot {
            overview: ParticipantOverview {
                case_id: inputs::case_id(),
                id: participant,
                revision,
                display_name: "Historical attendee".into(),
                procedural_role: "Declared role".into(),
                organization: None,
                directory_status: DirectoryStatus::Active,
                kind: None,
                subject: None,
            },
            values_digest: Sha256Digest::from_array([19; 32]),
        },
        subject_digest: None,
    });
    source.support = Some(support);
    inputs::resign_hearing(&mut source);
    hearing_result_receipt_matches(inputs::hasher().as_ref(), &source).unwrap();
    let source = DeadlineSourceDetail::HearingResult(Box::new(source));
    let (mut command, mut preparation) = fixture();
    let resolved = preparation.resolved.as_mut().unwrap();
    let mut profile_input = evaluation::definition();
    profile_input.trigger = TriggerRequirement::SourceField(TriggerField::HearingSessionEventTime);
    resolved.profile.definition = DeadlineProfileDefinition::new(profile_input).unwrap();
    resolved.profile.definition_digest =
        deadline_profile_definition_digest(inputs::hasher().as_ref(), &resolved.profile.definition);
    let profile_command = DeadlineProfileCommand {
        operation_id: resolved.profile.receipt.operation_id,
        profile_id: resolved.profile.id,
        change: DeadlineProfileChange::Publish {
            definition: resolved.profile.definition.clone(),
        },
    };
    resolved.profile.receipt.submission_digest = deadline_profile_submission_digest(
        inputs::hasher().as_ref(),
        resolved.profile.recorded_by.id,
        &profile_command,
        resolved.profile.algorithm,
        resolved.profile.definition_digest,
    );
    resolved.profile_head = resolved.profile.clone();
    let DeadlineChange::Register { definition } = &mut command.change else {
        unreachable!()
    };
    definition.input.selection = inputs::request(&source).trigger;
    resolved.material = inputs::material(source);
    let stored = detail(&prepare(command, preparation).unwrap());
    deadline_receipt_matches(inputs::hasher().as_ref(), &stored).unwrap();
    stored
}
