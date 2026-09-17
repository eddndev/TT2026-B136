use super::Fixture;
use crate::{
    deadline_profile_database_support as profiles, procedural_fact_backend_support as facts,
};
use application::{
    deadline_evaluations::*, deadline_profiles::*, deadlines::*, procedural_facts::*,
};
use domain::{
    deadline_triggers::{TriggerSelection, TriggerSourceRef},
    identity::Role,
    procedural_time::DeclaredProceduralTime,
};
pub use profiles::text;
use time::{Time, UtcOffset};
use uuid::Uuid;
pub fn label(value: &str) -> FactLabel {
    FactLabel::new(value).unwrap()
}
pub fn profile(db: &Fixture) -> DeadlineProfileDetail {
    let mut definition = profiles::input(Some(db.case));
    definition.completion = DeadlineCompletionPolicy::CivilCutoff(
        DeadlineCivilCutoff::new(
            Time::from_hms(17, 30, 0).unwrap(),
            UtcOffset::from_hms(-6, 0, 0).unwrap(),
            "2026-01-01".parse().unwrap(),
            "2026-12-31".parse().unwrap(),
            label("Synthetic filing channel"),
            Uuid::nil(),
        )
        .unwrap(),
    );
    let command = DeadlineProfileCommand {
        operation_id: DeadlineProfileOperationId::new(),
        profile_id: DeadlineProfileId::new(),
        change: DeadlineProfileChange::Publish {
            definition: DeadlineProfileDefinition::new(definition).unwrap(),
        },
    };
    profiles::persist(
        &profiles::service(db, db.owner, Role::Owner),
        DeadlineProfileCollection::ForCase(db.case),
        command,
    )
}
pub fn source(db: &Fixture) -> FactDetail {
    let original = facts::values("Declared resolution for deadline");
    let values = ResolutionValues::new(ResolutionValuesInput {
        class: original.class().clone(),
        subtype: None,
        issuer: original.issuer().clone(),
        issued_at: DeclaredProceduralTime::date("2026-01-06".parse().unwrap(), None).unwrap(),
        summary: original.summary().clone(),
        provenance: original.provenance().clone(),
    });
    facts::persist(
        &facts::service(db, db.owner, Role::Owner),
        db.case,
        ProceduralFactCommand::Resolution(ResolutionCommand::new(
            FactOperationId::new(),
            ResolutionId::new(),
            FactChange::record(values),
        )),
    )
}
pub fn command(
    db: &Fixture,
    profile: &DeadlineProfileDetail,
    source: &FactDetail,
) -> DeadlineCommand {
    DeadlineCommand {
        operation_id: DeadlineOperationId::new(),
        deadline_id: DeadlineId::new(),
        change: DeadlineChange::Register {
            definition: DeadlineDefinition {
                title: label("Declared response period"),
                profile: DeadlineProfileRef {
                    id: profile.id,
                    revision: profile.revision,
                },
                responsible: db.owner,
                input: DeadlineEvaluationInput {
                    selection: TriggerSelection {
                        case_id: db.case,
                        source: FactDeclaration::Known(TriggerSourceRef::Resolution(
                            facts::resolution_ref(source),
                        )),
                        qualification: None,
                    },
                    calendar: None,
                    ordered_quantity: None,
                    qualification: DeadlineApplicability {
                        statement: text(
                            "Operator declares applicability without proving legal effect",
                        ),
                        locator: label("Resolution page 1"),
                        scope_applies: FactDeclaration::Known(true),
                        unresolved_incident: FactDeclaration::Known(false),
                        conditions: profile
                            .definition
                            .conditions()
                            .iter()
                            .map(|condition| DeadlineConditionAnswer {
                                id: condition.id,
                                applies: FactDeclaration::Known(true),
                                locator: label("Declared applicability record"),
                            })
                            .collect(),
                    },
                },
            },
        },
    }
}
pub fn setup(db: &Fixture) -> DeadlineCommand {
    command(db, &profile(db), &source(db))
}
pub fn definition_mut(command: &mut DeadlineCommand) -> &mut DeadlineDefinition {
    match &mut command.change {
        DeadlineChange::Register { definition } | DeadlineChange::Correct { definition, .. } => {
            definition
        }
        _ => panic!("command has no definition"),
    }
}
pub fn correct(base: &DeadlineDetail) -> DeadlineCommand {
    let mut definition = base.definition.clone();
    definition.title = label("Corrected response period");
    DeadlineCommand {
        operation_id: DeadlineOperationId::new(),
        deadline_id: base.id,
        change: DeadlineChange::Correct {
            expected_revision: base.revision,
            definition,
            reason: text("Correct declared title"),
        },
    }
}
pub fn attention(base: &DeadlineDetail) -> DeadlineCommand {
    DeadlineCommand {
        operation_id: DeadlineOperationId::new(),
        deadline_id: base.id,
        change: DeadlineChange::SetAttention {
            expected_revision: base.revision,
            attention: DeadlineAttention::Recorded {
                occurred_at: DeclaredProceduralTime::unknown(),
                statement: text("Operator declares action; legal validity is not inferred"),
                locator: label("Filing reference"),
            },
            reason: text("Record the declared attention"),
        },
    }
}
pub fn retire(base: &DeadlineDetail) -> DeadlineCommand {
    DeadlineCommand {
        operation_id: DeadlineOperationId::new(),
        deadline_id: base.id,
        change: DeadlineChange::Retire {
            expected_revision: base.revision,
            reason: text("Retire tracking declaration"),
        },
    }
}
