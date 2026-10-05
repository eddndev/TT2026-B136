#![allow(dead_code)]

pub use crate::case_administration_support::Fixture;
use crate::case_stage_database_support::{
    creation, processor, upload, FixedClock, FormatCheck, TestIdentity,
};
use crate::typed_participant_service_support as typed;
pub use application::precautionary_measures::*;
use application::{
    case_stages::CaseStageEntry,
    cases::CaseRepository,
    documents::DocumentRecord,
    identity::Principal,
    precautionary_hearings::*,
    typed_participants::{
        ParticipantDetail, ParticipantSubmission, SubjectSnapshot, TypedParticipantWorkflow,
    },
};
pub use domain::precautionary_measures::*;
use domain::{
    cases::CaseId,
    crypto::DocumentVersionRef,
    hearings::{HearingNote, HearingSupportRef},
    identity::{Role, UserId},
    precautionary_hearings::MeasureId,
    procedural_time::DeclaredProceduralTime,
    typed_participants::SubjectRevisionRef,
};
use infrastructure::{PostgresMeasureDecisionStore, RingSha256Hasher};
use std::sync::Arc;

pub struct Seed {
    pub actor: Principal,
    pub command: MeasureDecisionCommand,
    pub subject: SubjectSnapshot,
    pub supervisor: ParticipantDetail,
    pub record: DocumentRecord,
}

pub fn principal(db: &mut Fixture, id: UserId) -> Principal {
    let row = db
        .admin
        .query_one("SELECT email,role FROM users WHERE id=$1", &[&id.as_uuid()])
        .unwrap();
    let role: String = row.get(1);
    Principal {
        id,
        email: row.get(0),
        role: match role.as_str() {
            "owner" => Role::Owner,
            "litigator" => Role::Litigator,
            "paralegal" => Role::Paralegal,
            "client" => Role::Client,
            _ => panic!("unknown fixture role"),
        },
    }
}

pub fn store(db: &Fixture) -> Arc<PostgresMeasureDecisionStore> {
    Arc::new(
        PostgresMeasureDecisionStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}

pub fn service(db: &Fixture, actor: Principal) -> MeasureDecisionService {
    service_with_format(db, actor, FormatCheck(None))
}

pub fn service_with_format(
    db: &Fixture,
    actor: Principal,
    format: FormatCheck,
) -> MeasureDecisionService {
    MeasureDecisionService::new(
        store(db),
        Arc::new(TestIdentity(actor)),
        processor(),
        Arc::new(format),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

pub fn reads(db: &Fixture, actor: Principal) -> MeasureDecisionReadService {
    MeasureDecisionReadService::new(
        store(db),
        Arc::new(TestIdentity(actor)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

pub fn confirmation(review: &MeasureDecisionReview) -> MeasureDecisionConfirmation {
    MeasureDecisionConfirmation {
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
    }
}

pub fn note(value: &str) -> HearingNote {
    HearingNote::new(value).unwrap()
}

pub fn declared_time() -> MeasureTime {
    MeasureTime::new(
        DeclaredProceduralTime::unknown(),
        Some(note("Time not declared")),
    )
    .unwrap()
}

pub fn values(subject: &SubjectSnapshot) -> MeasureValues {
    MeasureValues::new(MeasureValuesInput {
        subject: SubjectRevisionRef {
            id: subject.id,
            revision: subject.revision,
            values_digest: subject.values_digest,
        },
        kind: MeasureKind::PeriodicAppearance,
        conditions: note("Declared reporting terms"),
        validity: MeasureValidity::new(declared_time(), note("Declared validity"), None).unwrap(),
        supervision: MeasureSupervision::Unknown {
            reason: note("Supervisor not declared"),
        },
    })
}

pub fn impositions(subject: &SubjectSnapshot, count: usize) -> MeasureDecisionOutcome {
    MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(
        (0..count)
            .map(|_| {
                MeasureEffect::Impose(MeasureProposal {
                    id: MeasureId::new(),
                    values: values(subject),
                })
            })
            .collect(),
    ))
    .unwrap()
}

pub fn setup(db: &mut Fixture) -> Seed {
    db.case = CaseId::new();
    let case = db
        .store()
        .register_penal(db.owner, db.case, creation(&db.case.to_string()), db.at)
        .unwrap();
    let administration = case.administration.snapshot().unwrap().clone();
    let context = PrecautionaryContext::new(
        &RingSha256Hasher,
        PrecautionaryContextMaterial {
            case_id: db.case,
            administration: administration.clone(),
            stage: CaseStageEntry::Initial(case.initial_stage.unwrap()),
            stage_administration: administration,
        },
    )
    .unwrap();
    let record = upload(db, db.case, "measure-decision.pdf");
    let identities = typed::service(db, FormatCheck(None));
    let request = typed::reviewed(
        identities
            .review_participant("session", db.case, typed::proposal(&record))
            .unwrap(),
    );
    let supervisor = identities
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: request,
                signature: None,
            },
        )
        .unwrap();
    let subject = supervisor.bound_subject.clone().unwrap();
    let owner = db.owner;
    let actor = principal(db, owner);
    let command = MeasureDecisionCommand {
        operation_id: MeasureDecisionOperationId::new(),
        decision_id: MeasureDecisionId::new(),
        context: PrecautionaryContextExpectation {
            administration_revision: context.material().administration.revision,
            stage_revision: context.material().stage.stage_revision(),
            context_digest: context.digest(&RingSha256Hasher),
        },
        values: MeasureDecisionValues::new(MeasureDecisionValuesInput {
            authority: note("Declared court"),
            declared_at: declared_time(),
            justification: note("Declared reasons"),
            support: HearingSupportRef::new(
                DocumentVersionRef {
                    id: record.id,
                    version: record.version,
                },
                record.digest,
            ),
            locator: note("Page 1"),
        }),
        anchor: None,
        outcome: impositions(&subject, 2),
    };
    Seed {
        actor,
        command,
        subject,
        supervisor,
        record,
    }
}

pub fn persist(
    db: &Fixture,
    actor: Principal,
    command: MeasureDecisionCommand,
) -> MeasureDecisionStoredOperation {
    let workflow = service(db, actor);
    let review = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    workflow
        .submit("session", db.case, command, confirmation(&review))
        .unwrap()
}

pub fn fresh(command: &MeasureDecisionCommand) -> MeasureDecisionCommand {
    let mut command = command.clone();
    command.operation_id = MeasureDecisionOperationId::new();
    command.decision_id = MeasureDecisionId::new();
    command
}

pub fn no_change(command: &MeasureDecisionCommand) -> MeasureDecisionCommand {
    let mut command = fresh(command);
    command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::NoMeasureChange(
        note("No change was declared"),
    ))
    .unwrap();
    command
}

pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object('operations',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_measure_operations r),'decisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_measure_decisions r),'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_measures r),'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_measure_revisions r),'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
}

mod authorization;
mod lifecycle;
mod scope;

mod integrity;
mod schema;

mod commit_races;
mod schema_time;
