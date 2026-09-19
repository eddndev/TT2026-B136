use super::{profile, write, Fixture};
use application::{
    cases::CurrentCaseAdministration, deadline_evaluations::*,
    deadline_inputs::DeadlineInputMaterial, deadline_profiles::DeadlineProfileDetail, deadlines::*,
};
use domain::{
    cases::CaseMetadata,
    crypto::{DocumentHasher, Sha256Digest},
    deadline_triggers::TriggerSelection,
    identity::Role,
    procedural_facts::{FactDeclaration, FactLabel, FactText},
    procedural_time::DeclaredProceduralTime,
};
use infrastructure::RingSha256Hasher;

fn label(value: &str) -> FactLabel {
    FactLabel::new(value).unwrap()
}
fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}

// Construct Legacy explicitly so future human preparation changes cannot rewrite this fixture.
fn register(db: &Fixture, profile: &DeadlineProfileDetail) -> (DeadlineDetail, DeadlineCommand) {
    let input = DeadlineEvaluationInput {
        selection: TriggerSelection {
            case_id: db.case,
            source: FactDeclaration::Unknown(text("Exact source has not been supplied")),
            qualification: None,
        },
        calendar: None,
        ordered_quantity: None,
        qualification: DeadlineApplicability {
            statement: text("Synthetic applicability declaration"),
            locator: label("Fixture declaration"),
            scope_applies: FactDeclaration::Known(true),
            unresolved_incident: FactDeclaration::Known(false),
            conditions: profile
                .definition
                .conditions()
                .iter()
                .map(|value| DeadlineConditionAnswer {
                    id: value.id,
                    applies: FactDeclaration::Known(true),
                    locator: label("Fixture condition"),
                })
                .collect(),
        },
    };
    let material = DeadlineInputMaterial {
        case_id: db.case,
        administration: CurrentCaseAdministration::Unrevised(
            CaseMetadata::new("Baseline", "REF-OLD").unwrap(),
        ),
        source: None,
        source_head: None,
        calendar: None,
        calendar_head: None,
    };
    let evaluation =
        evaluate_profiled_deadline(&RingSha256Hasher, &profile.definition, &input, &material)
            .unwrap();
    let definition = DeadlineDefinition {
        title: label("Legacy response period"),
        profile: DeadlineProfileRef {
            id: profile.id,
            revision: profile.revision,
        },
        input,
        responsible: db.owner,
    };
    let command = DeadlineCommand {
        operation_id: DeadlineOperationId::new(),
        deadline_id: DeadlineId::new(),
        change: DeadlineChange::Register {
            definition: definition.clone(),
        },
    };
    let mut detail = DeadlineDetail {
        id: command.deadline_id,
        case_id: db.case,
        revision: DeadlineRevision::initial(),
        definition,
        calculation: DeadlineCalculation {
            profile: profile.clone(),
            material,
            result: DeadlineEvaluationRecord::capture(&evaluation),
        },
        tracking: None,
        responsible: DeadlineResponsibleSnapshot {
            id: db.owner,
            email: "owner@example.test".into(),
            role: Role::Owner,
        },
        attention: DeadlineAttention::Pending,
        status: DeadlineStatus::Active,
        reason: None,
        receipt: DeadlineReceipt {
            version: DeadlineReceiptVersion::Legacy,
            operation_id: command.operation_id,
            action: DeadlineAction::Register,
            expected_revision: 0,
            review_digest: Sha256Digest::from_array([0; 32]),
            capture_digest: Sha256Digest::from_array([0; 32]),
            submission_digest: Sha256Digest::from_array([0; 32]),
        },
        recorded_at: db.at,
        recorded_by: DeadlineActorSnapshot::User {
            id: db.owner,
            email: "owner@example.test".into(),
        },
    };
    sign(&mut detail, &command);
    (detail, command)
}

fn sign(value: &mut DeadlineDetail, command: &DeadlineCommand) {
    value.receipt.review_digest =
        RingSha256Hasher.hash_bytes(&deadline_review_bytes(&RingSha256Hasher, value).unwrap());
    value.receipt.capture_digest =
        RingSha256Hasher.hash_bytes(&deadline_capture_bytes(&RingSha256Hasher, value).unwrap());
    let bytes = deadline_submission_bytes(
        value.recorded_by.user_id().unwrap(),
        value.case_id,
        command,
        value.receipt.review_digest,
    );
    assert_eq!(&bytes[..5], b"DLTX1");
    value.receipt.submission_digest = RingSha256Hasher.hash_bytes(&bytes);
    deadline_receipt_matches(&RingSha256Hasher, value).unwrap();
}

fn followup(base: &DeadlineDetail, retire: bool) -> (DeadlineDetail, DeadlineCommand) {
    let reason = text(if retire {
        "Withdraw legacy deadline"
    } else {
        "Record declared attention"
    });
    let attention = DeadlineAttention::Recorded {
        occurred_at: DeclaredProceduralTime::unknown(),
        statement: text("A filing was declared; validity is not inferred"),
        locator: label("Declared filing reference"),
    };
    let command = DeadlineCommand {
        operation_id: DeadlineOperationId::new(),
        deadline_id: base.id,
        change: if retire {
            DeadlineChange::Retire {
                expected_revision: base.revision,
                reason: reason.clone(),
            }
        } else {
            DeadlineChange::SetAttention {
                expected_revision: base.revision,
                attention: attention.clone(),
                reason: reason.clone(),
            }
        },
    };
    let mut detail = base.clone();
    detail.revision = DeadlineRevision::new(base.revision.get() + 1).unwrap();
    detail.reason = Some(reason);
    detail.status = if retire {
        DeadlineStatus::Retired
    } else {
        DeadlineStatus::Active
    };
    if !retire {
        detail.attention = attention;
    }
    detail.receipt.operation_id = command.operation_id;
    detail.receipt.action = command.action();
    detail.receipt.expected_revision = base.revision.get();
    detail.recorded_at += time::Duration::seconds(1);
    sign(&mut detail, &command);
    (detail, command)
}

pub fn seed_history(db: &mut Fixture) -> Vec<DeadlineDetail> {
    let profile = profile::seed(db);
    let (first, command) = register(db, &profile);
    write::insert(db, &first, &command);
    let (second, command) = followup(&first, false);
    write::insert(db, &second, &command);
    let (third, command) = followup(&second, true);
    write::insert(db, &third, &command);
    let (active, command) = register(db, &profile);
    write::insert(db, &active, &command);
    vec![first, second, third, active]
}
