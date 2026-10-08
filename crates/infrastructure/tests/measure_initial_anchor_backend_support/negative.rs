use super::*;
use application::case_stages::{CaseStageWorkflow, DeclaredStageTime, StageTransition};
use domain::{
    case_administration::CaseStageRevision,
    crypto::Sha256Digest,
    hearings::{HearingId, HearingRevision},
};

#[test]
fn foreign_initial_hearing_cannot_anchor_a_decision_in_another_case() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let original_case = db.case;
    let foreign = setup(&mut db);
    assert_ne!(foreign.hearing.snapshot.case_id, original_case);
    db.case = original_case;
    seed.command.anchor = Some(anchor(&foreign.hearing));
    let workflow = service(&db, seed.actor);
    let before = snapshot(&mut db);

    assert!(workflow.prepare("session", db.case, seed.command).is_err());

    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn exact_initial_anchor_requires_both_digests_and_an_existing_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let workflow = service(&db, seed.actor);
    for field in ["values", "submission", "revision"] {
        let mut command = fresh(&seed.command);
        let Some(MeasureDecisionAnchorRef::Initial {
            revision,
            values_digest,
            submission_digest,
            ..
        }) = &mut command.anchor
        else {
            panic!("expected Initial anchor")
        };
        match field {
            "values" => {
                let mut bytes = *values_digest.as_bytes();
                bytes[0] ^= 1;
                *values_digest = Sha256Digest::from_array(bytes);
            }
            "submission" => {
                let mut bytes = *submission_digest.as_bytes();
                bytes[0] ^= 1;
                *submission_digest = Sha256Digest::from_array(bytes);
            }
            "revision" => *revision = HearingRevision::new(2).unwrap(),
            _ => unreachable!(),
        }
        let before = snapshot(&mut db);

        assert!(
            workflow.prepare("session", db.case, command).is_err(),
            "accepted {field}"
        );

        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn a_real_intermediate_hearing_is_rejected_even_with_valid_current_decision_context() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let record = crate::case_stage_database_support::upload(&db, db.case, "accusation.pdf");
    let stage =
        crate::case_stage_database_support::service(&db, db.owner, Role::Owner, FormatCheck(None))
            .transition(
                "session",
                db.case,
                CaseStageRevision::FIRST,
                StageTransition::to_intermediate(
                    DeclaredStageTime::instant(db.at).unwrap(),
                    crate::case_stage_database_support::reference(&record),
                    None,
                ),
            )
            .unwrap();
    let administration = db
        .store()
        .get_administration(db.owner, db.case, db.at)
        .unwrap()
        .administration
        .snapshot()
        .unwrap()
        .clone();
    let entry = stage.current.entry().unwrap().clone();
    let context = PrecautionaryContext::new(
        &RingSha256Hasher,
        PrecautionaryContextMaterial {
            case_id: db.case,
            administration: administration.clone(),
            stage: entry.clone(),
            stage_administration: administration.clone(),
        },
    )
    .unwrap();
    let old = &seed.hearing.snapshot.values;
    let values = HearingValues::new(HearingValuesInput {
        kind: HearingKind::Intermediate,
        scheduled_at: old.scheduled_at(),
        modality: old.modality(),
        venue: old.venue().clone(),
        note: None,
        participants: vec![],
        conviction_basis: None,
    })
    .unwrap();
    let intermediate = crate::hearing_database_support::persist(
        &hearing_service(&db),
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: HearingId::new(),
            change: HearingChange::Schedule {
                context: HearingContextExpectation {
                    case_revision: administration.revision,
                    stage_revision: entry.stage_revision(),
                },
                values,
            },
        },
    );
    assert_eq!(
        intermediate.snapshot.values.kind(),
        HearingKind::Intermediate
    );
    let mut command = no_change(&seed.command);
    command.context = PrecautionaryContextExpectation {
        administration_revision: administration.revision,
        stage_revision: entry.stage_revision(),
        context_digest: context.digest(&RingSha256Hasher),
    };
    command.anchor = None;
    let workflow = service(&db, seed.actor);
    workflow
        .prepare("session", db.case, command.clone())
        .expect("the current context and unanchored instruction are valid");
    command.anchor = Some(anchor(&intermediate));
    let before = snapshot(&mut db);

    assert!(workflow.prepare("session", db.case, command).is_err());

    assert_eq!(snapshot(&mut db), before);
}
