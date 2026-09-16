use application::{
    case_stages::StageSupportSnapshot, documents::*, procedural_facts::*,
    typed_participants::ParticipantOverview,
};
use domain::{
    cases::CaseId, crypto::*, hearing_results::*, hearings::HearingId, participants::*,
    procedural_time::DeclaredProceduralTime,
};
use time::{Date, Month, UtcOffset};
use uuid::Uuid;

pub fn digest(value: u8) -> Sha256Digest {
    Sha256Digest::from_array([value; 32])
}
pub fn empty() -> FactSources {
    FactSources {
        resolved: FactResolvedSources {
            resolution: None,
            participants: vec![],
            hearing_results: vec![],
        },
        views: FactSourceViews {
            resolution: None,
            participants: vec![],
            hearing_results: vec![],
        },
        direct_supports: vec![],
    }
}
pub fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}
pub fn add_resolution(sources: &mut FactSources) {
    let reference = FactResolutionRef {
        id: ResolutionId::from_uuid(Uuid::from_u128(2)),
        revision: FactRevision::initial(),
    };
    sources.resolved.resolution = Some(FactResolutionSourceSnapshot {
        case_id: case_id(),
        reference,
        values_digest: digest(3),
        submission_digest: digest(4),
        status: FactStatus::Recorded,
    });
    sources.views.resolution = Some(FactResolutionView {
        reference,
        class: FactDeclaration::Known(ResolutionClass::Order),
        issuer: FactDeclaration::Unknown(FactText::new("Unknown issuer").unwrap()),
        issued_at: DeclaredProceduralTime::unknown(),
        summary: FactText::new("Declared resolution").unwrap(),
    });
}
pub fn add_participant(sources: &mut FactSources, id: u128, revision: u32) {
    let reference = FactParticipantRef {
        id: ParticipantId::from_uuid(Uuid::from_u128(id)),
        revision: ParticipantRevision::new(revision).unwrap(),
    };
    sources.resolved.participants.push(FactParticipantSnapshot {
        case_id: case_id(),
        reference,
        values_digest: digest(5),
        status: DirectoryStatus::Active,
        subject: None,
    });
    sources.views.participants.push(ParticipantOverview {
        case_id: case_id(),
        id: reference.id,
        revision: reference.revision,
        display_name: "Declared name".into(),
        procedural_role: "Declared role".into(),
        organization: None,
        directory_status: DirectoryStatus::Active,
        kind: None,
        subject: None,
    });
}
pub fn add_hearing(sources: &mut FactSources, id: u128, agreement: Option<u128>) {
    let reference = FactHearingRef {
        hearing_id: HearingId::from_uuid(Uuid::from_u128(6)),
        result_id: HearingResultId::from_uuid(Uuid::from_u128(id)),
        revision: HearingResultRevision::initial(),
        agreement_id: agreement.map(|id| HearingResultAgreementId::from_uuid(Uuid::from_u128(id))),
    };
    sources
        .resolved
        .hearing_results
        .push(FactHearingSourceSnapshot {
            case_id: case_id(),
            reference,
            values_digest: digest(7),
            submission_digest: digest(8),
            status: HearingResultStatus::Recorded,
        });
    sources.views.hearing_results.push(FactHearingView {
        reference,
        occurrence: HearingResultOccurrence::Occurred,
        event_time: DeclaredHearingResultTime::date(
            Date::from_calendar_date(2026, Month::September, 16).unwrap(),
            UtcOffset::UTC,
        )
        .unwrap(),
        summary: HearingResultText::new("Declared hearing").unwrap(),
        agreement: reference.agreement_id.map(|id| {
            HearingResultAgreement::new(id, HearingResultText::new("Declared agreement").unwrap())
        }),
    });
}
pub fn add_support(sources: &mut FactSources, id: u128, version: u32) {
    sources.direct_supports.push(StageSupportSnapshot {
        reference: DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(id)),
            version: DocumentVersion::new(version).unwrap(),
        },
        digest: digest(9),
        name: "support.pdf".into(),
        format: StageDocumentFormat::Pdf,
        policy: StageFormatPolicy::PdfDocxV1,
    });
}
pub fn full() -> FactSources {
    let mut sources = empty();
    add_resolution(&mut sources);
    add_participant(&mut sources, 0, 1);
    add_participant(&mut sources, 0, 2);
    add_hearing(&mut sources, 0, None);
    add_hearing(&mut sources, 0, Some(0));
    add_support(&mut sources, 0, 1);
    add_support(&mut sources, 0, 2);
    sources
}
pub fn inconsistent(sources: &FactSources) {
    assert!(matches!(
        fact_sources_bytes(sources),
        Err(application::ApplicationError::ProceduralFact(
            ProceduralFactError::StoredInconsistent(_)
        ))
    ));
}

pub fn full_distinct_results() -> FactSources {
    let mut sources = full();
    let result = HearingResultId::from_uuid(Uuid::from_u128(1));
    sources.resolved.hearing_results[1].reference.result_id = result;
    sources.views.hearing_results[1].reference.result_id = result;
    sources
}
