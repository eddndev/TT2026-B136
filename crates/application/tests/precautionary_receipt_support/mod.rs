use application::case_stages::{StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot};
use application::cases::CaseRevision;
use application::identity::Principal;
use application::precautionary_hearings::*;
use application::ApplicationError;
use domain::cases::CaseId;
use domain::hearings::{HearingNote, HearingTime, HearingVenue};
use domain::identity::{Role, UserId};
use domain::participants::DirectoryStatus;
use domain::precautionary_hearings::*;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{context_support, participant_support};
pub use context_support::Hasher;

pub fn at() -> OffsetDateTime {
    context_support::at() + Duration::seconds(60)
}

pub fn context() -> PrecautionaryContext {
    PrecautionaryContext::new(&Hasher, context_support::initial()).unwrap()
}

pub fn later_context() -> PrecautionaryContext {
    let mut material = context_support::initial();
    material.administration.revision = CaseRevision::new(2).unwrap();
    material.administration.changed_at = at() + Duration::seconds(1);
    material.administration.changed_by.email = "later-administration@example.test".into();
    PrecautionaryContext::new(&Hasher, material).unwrap()
}

pub fn expectation(context: &PrecautionaryContext) -> PrecautionaryContextExpectation {
    PrecautionaryContextExpectation {
        administration_revision: context.material().administration.revision,
        stage_revision: context.material().stage.stage_revision(),
        context_digest: context.digest(&Hasher),
    }
}

pub fn values_input() -> PrecautionaryHearingValuesInput {
    let mut input = participant_support::hearing_input(&[(10, 1), (20, 2)]);
    input.scheduled_at =
        HearingTime::new((at() + Duration::days(1)).replace_nanosecond(0).unwrap()).unwrap();
    input
}

#[derive(Clone)]
pub struct Fixture {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: PrecautionaryHearingCommand,
    pub context: PrecautionaryContext,
    pub sources: PrecautionaryHearingSources,
}

impl Fixture {
    pub fn schedule() -> Self {
        let context = context();
        let values = PrecautionaryHearingValues::new(values_input()).unwrap();
        let selected = values.scheduling_basis().support();
        Self {
            actor: Principal {
                id: UserId::from_uuid(Uuid::from_u128(2)),
                email: "recording@example.test".into(),
                role: Role::Litigator,
            },
            case_id: context.material().case_id,
            command: PrecautionaryHearingCommand {
                operation_id: PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(30)),
                hearing_id: PrecautionaryHearingId::from_uuid(Uuid::from_u128(40)),
                change: PrecautionaryHearingChange::Schedule {
                    context: expectation(&context),
                    values,
                },
            },
            context,
            sources: PrecautionaryHearingSources {
                participants: vec![
                    participant_support::manual(10, 1, DirectoryStatus::Active),
                    participant_support::typed(20, 2, false, DirectoryStatus::Active),
                ],
                support: StageSupportSnapshot {
                    reference: selected.reference(),
                    digest: selected.digest(),
                    name: "appointment.pdf".into(),
                    format: StageDocumentFormat::Pdf,
                    policy: StageFormatPolicy::PdfDocxV1,
                },
            },
        }
    }

    pub fn replace(base: &PrecautionaryHearingCapture) -> Self {
        let mut fixture = Self::schedule();
        fixture.context = later_context();
        let mut input = values_input();
        input.venue = HearingVenue::new("Replacement court").unwrap();
        fixture.command.operation_id =
            PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(31));
        fixture.command.change = PrecautionaryHearingChange::Replace {
            expected_revision: base.review.result_revision,
            expected_capture_digest: base.capture_digest,
            context: expectation(&fixture.context),
            values: PrecautionaryHearingValues::new(input).unwrap(),
            reason: HearingNote::new("Correct the communicated venue").unwrap(),
        };
        fixture
    }

    pub fn cancel(base: &PrecautionaryHearingCapture) -> Self {
        let mut fixture = Self::schedule();
        fixture.context = later_context();
        fixture.sources = base.review.sources.clone();
        fixture.command.operation_id =
            PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(32));
        fixture.command.change = PrecautionaryHearingChange::Cancel {
            expected_revision: base.review.result_revision,
            expected_capture_digest: base.capture_digest,
            reason: HearingNote::new("Cancel the communicated appointment").unwrap(),
        };
        fixture
    }

    pub fn prepare(
        self,
        base: Option<&PrecautionaryHearingCapture>,
    ) -> Result<CheckedPrecautionaryHearingReview, ApplicationError> {
        prepare_precautionary_hearing_capture(
            &Hasher,
            &self.actor,
            self.case_id,
            self.command,
            self.context,
            self.sources,
            base,
        )
    }

    pub fn capture(
        self,
        base: Option<&PrecautionaryHearingCapture>,
        at: OffsetDateTime,
    ) -> PrecautionaryHearingCapture {
        self.prepare(base)
            .unwrap()
            .into_capture(&Hasher, at)
            .unwrap()
    }
}

pub fn scheduled() -> PrecautionaryHearingCapture {
    Fixture::schedule().capture(None, at())
}
