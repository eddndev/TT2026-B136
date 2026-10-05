#![allow(dead_code, unused_imports)]

pub use crate::case_administration_support::Fixture;
pub use crate::case_stage_database_support::{
    creation, processor, upload, FixedClock, FormatCheck, TestIdentity,
};
pub use application::precautionary_hearings::*;
pub use application::{case_stages::CaseStageEntry, cases::CaseRepository, identity::Principal};
pub use domain::precautionary_hearings::*;
pub use domain::{
    cases::CaseId,
    crypto::DocumentVersionRef,
    hearings::*,
    identity::{Role, UserId},
};
pub use infrastructure::{PostgresPrecautionaryHearingStore, RingSha256Hasher};
pub use std::sync::Arc;
pub use time::Duration;
#[path = "commands.rs"]
mod commands;
pub use commands::*;

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

pub fn store(db: &Fixture) -> Arc<PostgresPrecautionaryHearingStore> {
    Arc::new(
        PostgresPrecautionaryHearingStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}

pub fn service(db: &Fixture, actor: Principal) -> PrecautionaryHearingService {
    service_with_format(db, actor, FormatCheck(None))
}

pub fn service_with_format(
    db: &Fixture,
    actor: Principal,
    format: FormatCheck,
) -> PrecautionaryHearingService {
    PrecautionaryHearingService::new(
        store(db),
        Arc::new(TestIdentity(actor)),
        processor(),
        Arc::new(format),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

pub fn reads(db: &Fixture, actor: Principal) -> PrecautionaryHearingReadService {
    PrecautionaryHearingReadService::new(
        store(db),
        Arc::new(TestIdentity(actor)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

pub fn confirmation(review: &PrecautionaryHearingReview) -> PrecautionaryHearingConfirmation {
    PrecautionaryHearingConfirmation {
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
    }
}

pub fn expectation(context: &PrecautionaryContext) -> PrecautionaryContextExpectation {
    PrecautionaryContextExpectation {
        administration_revision: context.material().administration.revision,
        stage_revision: context.material().stage.stage_revision(),
        context_digest: context.digest(&RingSha256Hasher),
    }
}

pub fn setup(db: &mut Fixture) -> (Principal, PrecautionaryHearingCommand) {
    db.case = CaseId::new();
    let detail = db
        .store()
        .register_penal(db.owner, db.case, creation(&db.case.to_string()), db.at)
        .unwrap();
    let administration = detail.administration.snapshot().unwrap().clone();
    let context = PrecautionaryContext::new(
        &RingSha256Hasher,
        PrecautionaryContextMaterial {
            case_id: db.case,
            administration: administration.clone(),
            stage: CaseStageEntry::Initial(detail.initial_stage.unwrap()),
            stage_administration: administration,
        },
    )
    .unwrap();
    let record = upload(db, db.case, "precautionary-appointment.pdf");
    let values = PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
        purpose: PrecautionaryHearingPurpose::Imposition,
        scheduled_at: HearingTime::new((db.at + Duration::days(2)).replace_nanosecond(0).unwrap())
            .unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Court for the declared appointment").unwrap(),
        note: None,
        participants: vec![],
        scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
            HearingNote::new("Declared appointment order").unwrap(),
            HearingSupportRef::new(
                DocumentVersionRef {
                    id: record.id,
                    version: record.version,
                },
                record.digest,
            ),
            HearingNote::new("Page 1").unwrap(),
        ),
        review_targets: vec![],
    })
    .unwrap();
    let command = PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: PrecautionaryHearingId::new(),
        change: PrecautionaryHearingChange::Schedule {
            context: expectation(&context),
            values,
        },
    };
    let owner = db.owner;
    (principal(db, owner), command)
}

pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM case_precautionary_hearings h),'revisions',(SELECT jsonb_agg(to_jsonb(h) ORDER BY hearing_id,revision) FROM case_precautionary_hearing_revisions h),'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
}

pub fn persist(
    db: &Fixture,
    actor: Principal,
    command: PrecautionaryHearingCommand,
) -> PrecautionaryHearingStoredOperation {
    let workflow = service(db, actor);
    let draft = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    workflow
        .submit("session", db.case, command, confirmation(&draft))
        .unwrap()
}
