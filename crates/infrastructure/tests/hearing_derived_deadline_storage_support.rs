use crate::{
    case_administration_support::Fixture,
    case_stage_database_support::{FixedClock, TestIdentity},
    deadline_profile_database_support as profiles, hearing_database_support as hearings,
    hearing_result_database_support as results,
};
use application::{
    deadline_evaluations::*, deadline_profiles::*, deadline_reevaluation::*, deadline_tracking::*,
    deadlines::*, hearing_derived_deadlines::*, hearing_results::*, identity::Principal,
};
use domain::{
    audit::{chain_digest, AuditEvent, GENESIS_PREVIOUS},
    crypto::Sha256Digest,
    deadline_triggers::*,
    identity::Role,
    procedural_facts::*,
};
use infrastructure::{PostgresDeadlineStore, RingSha256Hasher};
use std::sync::Arc;

pub fn draft(db: &mut Fixture) -> HearingDerivedDeadlineDraft {
    hearings::complete(db);
    let hearing = hearings::persist(
        &hearings::service(db, db.owner, Role::Owner),
        db.case,
        hearings::schedule(),
    );
    let result_command = results::record(hearing.snapshot.id);
    let result_service = results::service(db, db.owner, Role::Owner);
    let result_draft = result_service
        .prepare("session", db.case, result_command.clone())
        .unwrap();
    let mut definition = profiles::input(Some(db.case));
    definition.trigger = TriggerRequirement::SourceField(TriggerField::HearingSessionEventTime);
    let profile = profiles::persist(
        &profiles::service(db, db.owner, Role::Owner),
        DeadlineProfileCollection::ForCase(db.case),
        DeadlineProfileCommand {
            operation_id: DeadlineProfileOperationId::new(),
            profile_id: DeadlineProfileId::new(),
            change: DeadlineProfileChange::Publish {
                definition: DeadlineProfileDefinition::new(definition).unwrap(),
            },
        },
    );
    let actor = Principal {
        id: db.owner,
        email: "owner@example.test".into(),
        role: Role::Owner,
    };
    let deadline_command = DeadlineHumanCommand::new(
        DeadlineCommand {
            operation_id: DeadlineOperationId::new(),
            deadline_id: DeadlineId::new(),
            change: DeadlineChange::Register {
                definition: DeadlineDefinition {
                    title: FactLabel::new("Declared consequence").unwrap(),
                    profile: DeadlineProfileRef {
                        id: profile.id,
                        revision: profile.revision,
                    },
                    responsible: db.owner,
                    input: DeadlineEvaluationInput {
                        selection: TriggerSelection {
                            case_id: db.case,
                            source: FactDeclaration::Known(TriggerSourceRef::HearingResult(
                                FactHearingRef {
                                    hearing_id: result_command.hearing_id,
                                    result_id: result_command.result_id,
                                    revision: HearingResultRevision::initial(),
                                    agreement_id: None,
                                },
                            )),
                            qualification: None,
                        },
                        calendar: None,
                        ordered_quantity: None,
                        qualification: DeadlineApplicability {
                            statement: profiles::text("Synthetic declared applicability"),
                            locator: FactLabel::new("Declared record").unwrap(),
                            scope_applies: FactDeclaration::Known(true),
                            unresolved_incident: FactDeclaration::Known(false),
                            conditions: profile
                                .definition
                                .conditions()
                                .iter()
                                .map(|c| DeadlineConditionAnswer {
                                    id: c.id,
                                    applies: FactDeclaration::Known(true),
                                    locator: FactLabel::new("Declared condition").unwrap(),
                                })
                                .collect(),
                        },
                    },
                },
            },
        },
        Some(TrackingPolicies {
            profile: TrackingPolicy::Follow,
            source: TrackingPolicy::Follow,
            calendar: TrackingPolicy::Undetermined,
        }),
    )
    .unwrap();
    prepare_hearing_derived_deadline(
        &RingSha256Hasher,
        &actor,
        db.case,
        HearingDerivedDeadlineCommand {
            result: result_command.clone(),
            deadline: deadline_command.clone(),
        },
        HearingDerivedDeadlineMaterial {
            result: result_draft,
            profile: profile.clone(),
            profile_head: profile,
            calendar: None,
            calendar_head: None,
            responsible: DeadlineResponsibleSnapshot {
                id: actor.id,
                email: actor.email.clone(),
                role: actor.role,
            },
        },
        db.at,
    )
    .unwrap()
}

pub fn record(db: &mut Fixture) -> HearingDerivedDeadlineRecord {
    let draft = draft(db);
    let actor = draft.actor().clone();
    let result_command = draft.command().result.clone();
    let deadline_command = draft.command().deadline.clone();
    let result_service = results::service(db, db.owner, Role::Owner);
    let result = results::persist(&result_service, db.case, result_command);
    let sequence:i64=db.admin.query_one("SELECT sequence FROM deadline_source_events WHERE source_kind='hearing_result' AND source_id=$1 AND revision=1",&[&result.snapshot.id.as_uuid()]).unwrap().get(0);
    let event = SourceEventReference {
        sequence: sequence as u64,
        family: DependencyFamily::HearingResult,
        source_id: result.snapshot.id.as_uuid(),
        revision: 1,
        case_id: Some(db.case),
        hearing_id: Some(result.snapshot.hearing_id.as_uuid()),
        operation_id: result.snapshot.receipt.operation_id.as_uuid(),
    };
    let creation =
        finalize_hearing_derived_deadline(&RingSha256Hasher, &draft, result, event).unwrap();
    let store = Arc::new(
        PostgresDeadlineStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    );
    let workflow = DeadlineService::new(
        store,
        Arc::new(TestIdentity(actor)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    );
    let prepared = workflow
        .prepare("session", db.case, deadline_command.clone())
        .unwrap();
    let actual = workflow
        .submit(
            "session",
            db.case,
            deadline_command,
            prepared.submission_digest,
        )
        .unwrap();
    assert_eq!(&actual, creation.deadline());
    restore_hearing_derived_deadline(&RingSha256Hasher, creation.evidence()).unwrap()
}

pub fn marker(record: &HearingDerivedDeadlineRecord) -> String {
    let evidence = record.evidence();
    format!(
        "hrdc1:operation:{}:capture:{}",
        evidence.command.result.operation_id,
        evidence.capture_digest.to_hex()
    )
}

/// Seeds consistent storage evidence directly; this is not a compound workflow.
pub fn insert(db: &mut Fixture, record: &HearingDerivedDeadlineRecord) {
    let mut client = db.runtime();
    let mut tx = client.transaction().unwrap();
    tx.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])
        .unwrap();
    let last = tx
        .query_opt(
            "SELECT sequence,chain FROM audit_events ORDER BY sequence DESC LIMIT 1",
            &[],
        )
        .unwrap();
    let (sequence, previous) = last
        .map(|r| {
            let bytes: Vec<u8> = r.get(1);
            (
                r.get::<_, i64>(0) + 1,
                Sha256Digest::from_array(bytes.try_into().unwrap()),
            )
        })
        .unwrap_or((0, GENESIS_PREVIOUS));
    let e = record.evidence();
    let event = AuditEvent::new(
        sequence as u64,
        e.deadline.recorded_at,
        &e.actor.email,
        "hearing_derived_deadline.registered",
        marker(record),
    );
    let chain = chain_digest(&RingSha256Hasher, &previous, &event).unwrap();
    let timestamp = event
        .timestamp
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    tx.execute("INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
        &[&sequence,&timestamp,&event.actor,&event.action,&event.resource,&chain.as_bytes().as_slice()]).unwrap();
    tx.execute("INSERT INTO case_hearing_derived_deadline_origins(operation_id,case_id,hearing_id,result_id,result_revision,deadline_id,deadline_revision,deadline_operation_id,source_event_sequence,actor_id,actor_email,actor_role,review_canonical,review_digest,capture_canonical,capture_digest,audit_sequence) VALUES($1,$2,$3,$4,1,$5,1,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
        &[&e.command.result.operation_id.as_uuid(),&e.result.snapshot.case_id.as_uuid(),&e.result.snapshot.hearing_id.as_uuid(),&e.result.snapshot.id.as_uuid(),&e.deadline.id.as_uuid(),&e.deadline.receipt.operation_id.as_uuid(),&(e.source_event.sequence as i64),&e.actor.id.as_uuid(),&e.actor.email,&e.actor.role.as_str(),&record.review_bytes(),&e.review_digest.as_bytes().as_slice(),&record.capture_bytes(),&e.capture_digest.as_bytes().as_slice(),&sequence]).unwrap();
    tx.commit().unwrap();
}
