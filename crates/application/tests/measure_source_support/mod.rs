use application::participants::ParticipantActorSnapshot;
use application::precautionary_measures::{
    resolve_measure_sources, MeasureSourceProjection, MeasureSources,
};
use application::typed_participants::{
    subject_digest, typed_participant_digest, ParticipantDetail, SubjectRevisionRef,
    SubjectSnapshot, TypedParticipantValues,
};
use application::ApplicationError;
use domain::clock::OffsetDateTime;
use domain::hearings::{HearingNote, HearingParticipantRef};
use domain::participants::DirectoryStatus;
use domain::precautionary_measures::{
    MeasureKind, MeasureSupervision, MeasureTime, MeasureValidity, MeasureValues,
    MeasureValuesInput,
};
use domain::procedural_time::DeclaredProceduralTime;

pub use crate::participant_support::{
    case_id, manual, manual_mut, other_case, reference, subject_values, typed, typed_mut, Hasher,
};

pub struct Fixture {
    pub values: MeasureValues,
    pub sources: MeasureSources,
}

impl Fixture {
    pub fn unknown(institutional: bool) -> Self {
        Self::new(institutional, None)
    }

    pub fn manual(status: DirectoryStatus) -> Self {
        Self::new(false, Some(manual(7, 3, status)))
    }

    pub fn typed(institutional: bool, status: DirectoryStatus) -> Self {
        Self::new(false, Some(typed(7, 3, institutional, status)))
    }

    pub fn shared_subject(institutional: bool) -> Self {
        let mut fixture = Self::typed(institutional, DirectoryStatus::Archived);
        fixture.sources.subject = fixture
            .sources
            .supervisor
            .as_ref()
            .unwrap()
            .bound_subject
            .clone()
            .unwrap();
        fixture.values = MeasureValues::new(input(&fixture.sources));
        fixture
    }

    fn new(institutional: bool, supervisor: Option<ParticipantDetail>) -> Self {
        let mut subject = typed(9, 1, institutional, DirectoryStatus::Active)
            .bound_subject
            .unwrap();
        subject.id =
            application::typed_participants::CaseSubjectId::from_uuid(uuid::Uuid::from_u128(60));
        let sources = MeasureSources {
            subject,
            supervisor,
        };
        Self {
            values: MeasureValues::new(input(&sources)),
            sources,
        }
    }

    pub fn resolve(&self) -> Result<MeasureSourceProjection, ApplicationError> {
        resolve_measure_sources(&Hasher, case_id(), &self.values, &self.sources)
    }

    pub fn supervisor_mut(&mut self) -> &mut ParticipantDetail {
        self.sources.supervisor.as_mut().unwrap()
    }
}

pub fn input(sources: &MeasureSources) -> MeasureValuesInput {
    MeasureValuesInput {
        subject: subject_ref(&sources.subject),
        kind: MeasureKind::PeriodicAppearance,
        conditions: HearingNote::new("Appear as stated in the source").unwrap(),
        validity: MeasureValidity::new(
            MeasureTime::new(
                DeclaredProceduralTime::unknown(),
                Some(HearingNote::new("Start not stated").unwrap()),
            )
            .unwrap(),
            HearingNote::new("Validity as declared in the source").unwrap(),
            None,
        )
        .unwrap(),
        supervision: match &sources.supervisor {
            Some(detail) => MeasureSupervision::Known {
                participant: HearingParticipantRef::new(detail.id(), detail.revision_number()),
                statement: HearingNote::new("Supervisor identified in the source").unwrap(),
            },
            None => MeasureSupervision::Unknown {
                reason: HearingNote::new("Supervisor not stated").unwrap(),
            },
        },
    }
}

pub fn subject_ref(subject: &SubjectSnapshot) -> SubjectRevisionRef {
    SubjectRevisionRef {
        id: subject.id,
        revision: subject.revision,
        values_digest: subject.values_digest,
    }
}

pub fn rebind_supervisor(detail: &mut ParticipantDetail) {
    let subject = detail.bound_subject.as_mut().unwrap();
    subject.values_digest = subject_digest(&Hasher, &subject.values);
    let reference = subject_ref(subject);
    let snapshot = typed_mut(detail);
    snapshot.values = TypedParticipantValues::new(
        reference,
        snapshot.values.directory_status(),
        snapshot.values.role().clone(),
    );
    snapshot.values_digest = typed_participant_digest(&Hasher, &snapshot.values);
}

pub fn provenance(
    fixture: &mut Fixture,
    location: usize,
) -> (&mut OffsetDateTime, &mut ParticipantActorSnapshot) {
    match location {
        0 => {
            let subject = &mut fixture.sources.subject;
            (&mut subject.changed_at, &mut subject.changed_by)
        }
        1 => {
            let participant = manual_mut(fixture.supervisor_mut());
            (&mut participant.changed_at, &mut participant.changed_by)
        }
        2 => {
            let participant = typed_mut(fixture.supervisor_mut());
            (&mut participant.changed_at, &mut participant.changed_by)
        }
        _ => {
            let subject = fixture.supervisor_mut().bound_subject.as_mut().unwrap();
            (&mut subject.changed_at, &mut subject.changed_by)
        }
    }
}

pub fn provenance_fixture(location: usize) -> Fixture {
    if location == 1 {
        Fixture::manual(DirectoryStatus::Archived)
    } else {
        Fixture::typed(false, DirectoryStatus::Archived)
    }
}
