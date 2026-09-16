use super::super::support::{digest, notification};
use application::{
    case_stages::StageSupportSnapshot,
    documents::{StageDocumentFormat, StageFormatPolicy},
    procedural_facts::*,
    typed_participants::ParticipantOverview,
};
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    hearing_results::*,
    hearings::HearingId,
    participants::{DirectoryStatus, ParticipantId, ParticipantRevision},
    typed_participants::{CaseSubjectId, ParticipantKind, SubjectRevision, SubjectRevisionRef},
};
use uuid::Uuid;

pub(super) fn rich_detail() -> FactDetail {
    let mut row = notification();
    let case = row.snapshot.case_id();
    let reference = FactParticipantRef {
        id: ParticipantId::new(),
        revision: ParticipantRevision::new(7).unwrap(),
    };
    let subject = SubjectRevisionRef {
        id: CaseSubjectId::new(),
        revision: SubjectRevision::new(3).unwrap(),
        values_digest: digest(8),
    };
    row.sources
        .resolved
        .participants
        .push(FactParticipantSnapshot {
            case_id: case,
            reference,
            values_digest: digest(6),
            status: DirectoryStatus::Archived,
            subject: Some(subject),
        });
    row.sources.views.participants.push(ParticipantOverview {
        case_id: case,
        id: reference.id,
        revision: reference.revision,
        display_name: "Historical counsel".into(),
        procedural_role: "defense_counsel".into(),
        organization: Some("Office".into()),
        directory_status: DirectoryStatus::Archived,
        kind: Some(ParticipantKind::DefenseCounsel),
        subject: Some(subject),
    });
    let hearing = FactHearingRef {
        hearing_id: HearingId::new(),
        result_id: HearingResultId::new(),
        revision: HearingResultRevision::new(2).unwrap(),
        agreement_id: Some(HearingResultAgreementId::from_uuid(Uuid::nil())),
    };
    row.sources
        .resolved
        .hearing_results
        .push(FactHearingSourceSnapshot {
            case_id: case,
            reference: hearing,
            values_digest: digest(10),
            submission_digest: digest(11),
            status: HearingResultStatus::Withdrawn,
        });
    row.sources.views.hearing_results.push(FactHearingView {
        reference: hearing,
        occurrence: HearingResultOccurrence::Occurred,
        event_time: DeclaredHearingResultTime::date(
            time::Date::from_calendar_date(2026, time::Month::September, 16).unwrap(),
            time::UtcOffset::from_hms(-6, 0, 0).unwrap(),
        )
        .unwrap(),
        summary: HearingResultText::new("Historical result").unwrap(),
        agreement: Some(HearingResultAgreement::new(
            hearing.agreement_id.unwrap(),
            HearingResultText::new("Exact selected agreement").unwrap(),
        )),
    });
    let document = DocumentVersionRef {
        id: DocumentId::new(),
        version: DocumentVersion::new(3).unwrap(),
    };
    row.sources.direct_supports.push(StageSupportSnapshot {
        reference: document,
        digest: digest(12),
        name: "notice.pdf".into(),
        format: StageDocumentFormat::Pdf,
        policy: StageFormatPolicy::PdfDocxV1,
    });
    let ProceduralFactSnapshot::Notification(snapshot) = &mut row.snapshot else {
        unreachable!()
    };
    let previous = &snapshot.values;
    snapshot.values = NotificationValues::new(NotificationValuesInput {
        resolution: previous.resolution(),
        character: previous.character().clone(),
        medium: previous.medium().clone(),
        context: previous.context().clone(),
        outcome: previous.outcome().clone(),
        subtype: None,
        practiced_at: previous.practiced_at(),
        received_at: previous.received_at(),
        stated_effect: None,
        intended_recipient: FactDeclaration::Known(FactPerson::Participant(reference)),
        actual_receiver: previous.actual_receiver().clone(),
        representation: previous.representation().clone(),
        summary: previous.summary().clone(),
        provenance: FactProvenance::HearingResult {
            reference: hearing,
            locator: FactLabel::new("Agreement location").unwrap(),
            support: Some(FactEvidence::new(
                document,
                digest(12),
                FactLabel::new("Page 1").unwrap(),
            )),
        },
    })
    .unwrap();
    row
}
