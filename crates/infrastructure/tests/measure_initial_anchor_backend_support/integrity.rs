use super::*;
use application::{documents::StageSupportReadLimits, participants::ParticipantStore};
use domain::{
    hearings::HearingParticipantRef,
    participants::{DirectoryStatus, ParticipantId, ParticipantValues},
};
use postgres::{Client, NoTls};

mod support;
use support::*;

#[test]
fn missing_older_initial_prefix_or_root_rejects_existing_reads_and_replay() {
    for missing in ["prefix", "root"] {
        let Some(mut db) = Fixture::new() else { return };
        let mut seed = setup(&mut db);
        let selected = super::history::replace(&db, &seed.hearing);
        seed.command.anchor = Some(anchor(&selected));
        let original = persist(&db, seed.actor.clone(), seed.command.clone());
        let storage = store(&db);
        assert_eq!(
            storage
                .get(&seed.actor, db.case, original.origin.decision_id)
                .unwrap(),
            original
        );
        let sql = if missing == "prefix" {
            "DELETE FROM case_hearing_revisions WHERE revision=1"
        } else {
            "DELETE FROM case_hearings"
        };
        damage(
            &mut db.admin,
            &["case_hearings", "case_hearing_revisions"],
            sql,
        );
        let before = snapshot(&mut db);

        reject_original(&storage, &seed.actor, db.case, &seed.command);
        assert!(storage
            .prepare(
                &seed.actor,
                db.case,
                &no_change(&seed.command),
                &StageSupportReadLimits::default()
            )
            .is_err());
        assert!(open(&db).is_err(), "accepted a lost ordinary {missing}");
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn missing_or_altered_original_prefix_audit_rejects_the_selected_later_anchor() {
    for alter in [false, true] {
        let Some(mut db) = Fixture::new() else { return };
        let mut seed = setup(&mut db);
        let selected = super::history::replace(&db, &seed.hearing);
        seed.command.anchor = Some(anchor(&selected));
        persist(&db, seed.actor.clone(), seed.command.clone());
        let storage = store(&db);
        let original_marker = marker(&seed.hearing);
        let sequence: i64 = db
            .admin
            .query_one(
                "SELECT sequence FROM audit_events WHERE resource=$1",
                &[&original_marker],
            )
            .unwrap()
            .get(0);
        let sql = if alter {
            format!(
                "UPDATE audit_events SET resource=resource||':altered' WHERE sequence={sequence}"
            )
        } else {
            format!("DELETE FROM audit_events WHERE sequence={sequence}")
        };
        damage(&mut db.admin, &["audit_events"], &sql);
        let before = snapshot(&mut db);

        reject_original(&storage, &seed.actor, db.case, &seed.command);
        assert!(open(&db).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn duplicate_original_hearing_marker_is_rejected_even_with_a_valid_new_chain_link() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), seed.command.clone());
    let storage = store(&db);
    assert_eq!(
        storage
            .get(&seed.actor, db.case, original.origin.decision_id)
            .unwrap(),
        original
    );
    duplicate_audit(&db, &seed.hearing);
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE resource=$1",
            &[&marker(&seed.hearing)],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 2);
    let before = snapshot(&mut db);

    reject_original(&storage, &seed.actor, db.case, &seed.command);
    assert!(open(&db).is_err());
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn changed_exact_historical_participant_values_cannot_rewrite_the_captured_initial_anchor() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let participants =
        infrastructure::PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher))
            .unwrap();
    let participant = participants
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            ParticipantValues::new(
                "Original historical witness",
                "Witness",
                None,
                None,
                DirectoryStatus::Active,
            )
            .unwrap(),
            db.at,
        )
        .unwrap();
    let mut appointment = crate::hearing_database_support::schedule();
    let HearingChange::Schedule { values, .. } = &mut appointment.change else {
        unreachable!()
    };
    *values = HearingValues::new(HearingValuesInput {
        kind: values.kind(),
        scheduled_at: values.scheduled_at(),
        modality: values.modality(),
        venue: values.venue().clone(),
        note: values.note().cloned(),
        participants: vec![HearingParticipantRef::new(
            participant.id,
            participant.revision,
        )],
        conviction_basis: values.conviction_basis().cloned(),
    })
    .unwrap();
    let selected =
        crate::hearing_database_support::persist(&hearing_service(&db), db.case, appointment);
    seed.command.anchor = Some(anchor(&selected));
    let original = persist(&db, seed.actor.clone(), seed.command.clone());
    assert_anchor(&original, &selected);
    let storage = store(&db);
    assert_eq!(
        storage
            .get(&seed.actor, db.case, original.origin.decision_id)
            .unwrap(),
        original
    );
    damage(
        &mut db.admin,
        &["case_participant_revisions"],
        "UPDATE case_participant_revisions SET display_name='Changed historical witness',
         values_digest=sha256(participant_values_bytes('Changed historical witness',procedural_role,
             organization,legal_status,directory_status))",
    );
    let before = snapshot(&mut db);

    reject_original(&storage, &seed.actor, db.case, &seed.command);
    assert!(open(&db).is_err());
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn losing_the_initial_anchor_root_during_support_admission_leaves_no_group_or_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let draft = service(&db, seed.actor.clone())
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    let url = db.admin_url.clone();
    let workflow = service_with_format(
        &db,
        seed.actor,
        FormatCheck(Some(Box::new(move || {
            let mut admin = Client::connect(&url, NoTls).unwrap();
            damage(
                &mut admin,
                &["case_hearings", "case_hearing_revisions"],
                "DELETE FROM case_hearings",
            );
        }))),
    );
    let before = snapshot(&mut db);

    assert!(workflow
        .submit("session", db.case, seed.command, confirmation(&draft))
        .is_err());

    assert_eq!(snapshot(&mut db), before);
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_hearings", &[])
            .unwrap()
            .get::<_, i64>(0),
        0,
        "the admission hook must actually remove the anchor root"
    );
}
