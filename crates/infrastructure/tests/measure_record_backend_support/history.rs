use super::*;
use application::{
    cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation},
    participants::{DirectoryStatus, ParticipantStore},
};
use domain::{hearings::HearingParticipantRef, precautionary_measures::*};
use infrastructure::PostgresParticipantStore;

#[test]
fn reopened_record_reads_preserve_original_sources_and_actor_after_archive_and_closure() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = crate::measure_fixture::setup(&mut db);
    let prior = crate::measure_fixture::values(&seed.subject);
    let values = MeasureValues::new(MeasureValuesInput {
        subject: prior.subject(),
        kind: prior.kind(),
        conditions: prior.conditions().clone(),
        validity: prior.validity().clone(),
        supervision: MeasureSupervision::Known {
            participant: HearingParticipantRef::new(
                seed.supervisor.id(),
                seed.supervisor.revision_number(),
            ),
            statement: crate::measure_fixture::note("Exact original supervisor"),
        },
    });
    seed.command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Impose(MeasureProposal {
            id: MeasureId::new(),
            values,
        }),
    ]))
    .unwrap();
    let group = crate::measure_fixture::persist(&db, seed.actor.clone(), seed.command.clone());
    let user = db.user("litigator", true);
    let recorded_actor = crate::measure_fixture::principal(&mut db, user);
    let original = crate::administrative_fixture::persist(
        &db,
        recorded_actor.clone(),
        crate::administrative_fixture::correction(
            crate::administrative_fixture::reference(&group.group.measures[0]),
            seed.command.context,
            &group.group.measures[0].result.values,
            "Corrected source transcription",
        ),
    );
    PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher))
        .unwrap()
        .change_status(
            db.owner,
            db.case,
            seed.supervisor.id(),
            seed.supervisor.revision_number(),
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    db.admin.execute("UPDATE users SET email='record-reader@example.test',role='paralegal',revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1", &[&user.as_uuid()]).unwrap();
    let reader = crate::measure_fixture::principal(&mut db, user);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(seed.command.context.administration_revision),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let service = reads(&db, reader.clone());
    let expected = administrative(&original);
    same_detail(
        &service
            .get("session", db.case, expected.reference.id())
            .unwrap(),
        &expected,
    );
    same_detail(
        &service
            .exact("session", db.case, judicial(&group, 0).reference)
            .unwrap(),
        &judicial(&group, 0),
    );
    let page = service
        .list("session", db.case, MeasureRecordReadQuery::default())
        .unwrap();
    assert_eq!(page.items.len(), 1);
    same_detail(&page.items[0], &expected);
    assert_eq!(original.capture.review.actor, recorded_actor);
    assert_eq!(
        original.capture.review.result.sources.supervisor.as_ref(),
        Some(&seed.supervisor)
    );
    assert_eq!(original.capture.review.result.sources.subject, seed.subject);
    let accesses = db.admin.query("SELECT actor,action FROM audit_events WHERE action IN ('measure_record.list','measure_record.read','measure_record.exact') ORDER BY sequence", &[]).unwrap();
    assert_eq!(accesses.len(), 3);
    assert!(accesses
        .iter()
        .all(|row| row.get::<_, String>("actor") == reader.email));
    assert_eq!(
        accesses
            .iter()
            .map(|row| row.get::<_, String>("action"))
            .collect::<Vec<_>>(),
        [
            "measure_record.read",
            "measure_record.exact",
            "measure_record.list"
        ]
    );
}

#[test]
fn current_terminal_declaration_and_its_original_revision_remain_readable() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = crate::measure_fixture::setup(&mut db);
    let initial = crate::measure_fixture::persist(&db, seed.actor.clone(), seed.command.clone());
    let previous = crate::administrative_fixture::reference(&initial.group.measures[0]);
    let terminal = crate::measure_fixture::persist(
        &db,
        seed.actor.clone(),
        crate::administrative_fixture::effect_command(
            &seed.command,
            vec![MeasureEffect::Revoke { previous }],
        ),
    );
    let service = reads(&db, seed.actor);
    let expected = judicial(&terminal, 0);
    same_detail(
        &service.get("session", db.case, previous.id()).unwrap(),
        &expected,
    );
    same_detail(
        &service
            .exact("session", db.case, expected.reference)
            .unwrap(),
        &expected,
    );
    same_detail(
        &service.exact("session", db.case, previous).unwrap(),
        &judicial(&initial, 0),
    );
    assert_eq!(
        terminal.group.measures[0].result.action,
        MeasureCaptureAction::Revoke
    );
    let page = service
        .list("session", db.case, MeasureRecordReadQuery::default())
        .unwrap();
    assert_eq!(page.items.len(), 2);
    same_detail(
        page.items
            .iter()
            .find(|row| row.reference.id() == previous.id())
            .unwrap(),
        &expected,
    );
}
