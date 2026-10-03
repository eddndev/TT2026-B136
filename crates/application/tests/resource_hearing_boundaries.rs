use super::*;
use application::{cases::*, participants::*, typed_participants::ParticipantRevisionSnapshot};
use domain::participants::{ParticipantId, ParticipantRevision, ParticipantValues};

fn with_people(count: usize) -> Fixture {
    let mut fixture = Fixture::new(Role::Owner);
    for id in 1..=count {
        let values = ParticipantValues::new(
            "Selected person",
            "Declared role",
            None,
            None,
            DirectoryStatus::Active,
        )
        .unwrap();
        fixture.material.participants.push(
            ParticipantSnapshot {
                case_id: fixture.case(),
                id: ParticipantId::from_uuid(uuid::Uuid::from_u128(id as u128)),
                revision: ParticipantRevision::initial(),
                values_digest: participant_digest(hearing_support::hasher().as_ref(), &values),
                values,
                changed_at: case_support::instant(),
                changed_by: ParticipantActorSnapshot {
                    id: fixture.actor.id,
                    email: fixture.actor.email.clone(),
                },
            }
            .into(),
        );
    }
    let references = fixture
        .material
        .participants
        .iter()
        .map(|person| HearingParticipantRef::new(person.id(), person.revision_number()))
        .collect();
    fixture.change_values(|input| input.participants = references);
    fixture
}

#[test]
fn all_32_participants_are_verified_and_ordered_without_changing_the_fact_limit() {
    let mut fixture = with_people(32);
    let original = fixture.prepare().unwrap();
    assert_eq!(original.participants.len(), 32);
    fixture.material.participants.reverse();
    assert_eq!(original, fixture.prepare().unwrap());
    let mut changed = with_people(1);
    let first = changed.prepare().unwrap();
    let ParticipantRevisionSnapshot::Manual(person) =
        &mut changed.material.participants[0].revision
    else {
        unreachable!()
    };
    person.values = ParticipantValues::new(
        "Different stored person",
        "Declared role",
        None,
        None,
        DirectoryStatus::Active,
    )
    .unwrap();
    person.values_digest = participant_digest(hearing_support::hasher().as_ref(), &person.values);
    assert_ne!(
        first.submission_digest,
        changed.prepare().unwrap().submission_digest
    );
}

#[test]
fn foreign_archived_missing_or_corrupt_participant_material_is_rejected() {
    for mutation in 0..4 {
        let mut fixture = with_people(1);
        let ParticipantRevisionSnapshot::Manual(person) =
            &mut fixture.material.participants[0].revision
        else {
            unreachable!()
        };
        match mutation {
            0 => person.case_id = domain::cases::CaseId::new(),
            1 => {
                person.values = ParticipantValues::new(
                    "Archived",
                    "Declared role",
                    None,
                    None,
                    DirectoryStatus::Archived,
                )
                .unwrap();
                person.values_digest =
                    participant_digest(hearing_support::hasher().as_ref(), &person.values);
            }
            2 => person.values_digest = Sha256Digest::from_array([7; 32]),
            _ => fixture.material.participants.clear(),
        }
        assert!(fixture.prepare_rejected().is_err());
    }
}

#[test]
fn closed_case_blocks_review_even_when_the_resource_remains_active() {
    let mut fixture = Fixture::new(Role::Owner);
    let values = fixture
        .material
        .administration
        .values()
        .with_status(CaseAdministrativeStatus::Closed);
    let revision = fixture
        .material
        .administration
        .revision()
        .map(|value| value.next().unwrap())
        .unwrap_or(CaseRevision::FIRST);
    fixture.material.administration =
        CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
            case_id: fixture.case(),
            revision,
            values_digest: case_administration_digest(hearing_support::hasher().as_ref(), &values),
            values,
            changed_at: case_support::instant(),
            changed_by: CaseActorSnapshot {
                id: fixture.actor.id,
                email: fixture.actor.email.clone(),
            },
        }));
    assert!(matches!(
        fixture.prepare_rejected(),
        Err(ApplicationError::CaseClosed)
    ));
}

#[test]
fn foreign_preparation_scope_is_rejected_before_a_review_is_returned() {
    let mut fixture = Fixture::new(Role::Owner);
    fixture.material.resource_head.case_id = domain::cases::CaseId::new();
    assert!(matches!(
        fixture.prepare_rejected(),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::SourceMismatch
        ))
    ));
}
