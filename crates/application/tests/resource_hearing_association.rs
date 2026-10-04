use super::*;
use application::resource_hearings::*;

fn creation(f: &Fixture) -> ResourceHearingCreation {
    prepare_resource_hearing_change(
        hearing_support::hasher(),
        &f.actor,
        f.case(),
        f.resource(),
        f.command.clone(),
        f.material.clone(),
    )
    .unwrap()
    .into_creation(case_support::instant())
    .unwrap()
}

fn rehash(row: &mut ResourceActivityDetail) {
    let hasher = hearing_support::hasher();
    let draft = ResourceActivityDraft {
        case_id: row.case_id,
        resource_id: row.resource_id,
        command: resource_activity_command_from_detail(row).unwrap(),
        result_revision: row.revision,
        selection: row.selection,
        status: row.status,
        sources: row.sources.clone(),
        previous: row.receipt.previous,
        recorded_by: row.recorded_by.clone(),
        observed_administration: row.recorded_administration.clone(),
        observed_resource_head: row.recorded_resource_head,
        submission_digest: row.receipt.submission_digest,
    };
    row.receipt.submission_digest =
        hasher.hash_bytes(&resource_activity_submission_bytes(hasher.as_ref(), &draft).unwrap());
    row.receipt.capture_digest = hasher.hash_bytes(&resource_activity_capture_bytes(row));
}

#[test]
fn creation_contains_a_verified_initial_association_to_its_exact_hearing() {
    let f = Fixture::new(Role::Owner);
    let result = creation(&f);
    let hearing = &result.hearing;
    let row = &result.association;
    resource_hearing_receipt_matches(hearing_support::hasher().as_ref(), hearing).unwrap();
    resource_activity_receipt_matches(hearing_support::hasher().as_ref(), row).unwrap();
    resource_hearing_creation_matches(hearing_support::hasher().as_ref(), &result).unwrap();
    assert_eq!((row.case_id, row.resource_id), (f.case(), f.resource()));
    assert_eq!(row.id, f.command.association_id);
    assert_eq!(row.revision, ResourceActivityRevision::initial());
    assert_eq!(row.status, ResourceActivityStatus::Linked);
    assert_eq!(row.receipt.action, ResourceActivityAction::Link);
    assert_eq!(row.receipt.expected_revision, 0);
    assert_eq!(row.receipt.previous, None);
    assert_eq!(row.reason, None);
    assert_eq!(row.selection.resource, f.command.resource);
    assert_eq!(row.selection.act, f.command.act);
    assert_eq!(row.sources.resource, f.material.resource);
    assert_eq!(row.sources.act, f.material.act);
    assert_eq!(row.recorded_at, hearing.recorded_at);
    assert_eq!(row.recorded_by, hearing.review.recorded_by);
    assert_eq!(row.recorded_administration, f.material.administration);
    assert_eq!(
        row.recorded_resource_head,
        hearing.review.observed_resource_head
    );
    assert_eq!(
        row.receipt.expected_resource_revision,
        f.command.expected_resource_revision
    );
    assert_eq!(
        row.receipt.operation_id.as_uuid(),
        f.command.operation_id.as_uuid()
    );
    assert_eq!(
        row.selection.target,
        ResourceActivityTarget::ResourceHearing {
            id: f.command.hearing_id,
            revision: hearing.revision,
            capture_digest: hearing.capture_digest,
        }
    );
    assert_eq!(
        row.sources.target,
        ResourceActivityTargetDetail::ResourceHearing(Box::new(hearing.clone()))
    );
    assert_eq!(result.origin.association_id, row.id);
}

#[test]
fn creation_rejects_individually_valid_association_substitutions() {
    let f = Fixture::new(Role::Owner);
    let result = creation(&f);
    for fault in 0..7 {
        let mut changed = result.clone();
        let row = &mut changed.association;
        match fault {
            0 => row.id = ResourceActivityId::new(),
            1 => row.receipt.operation_id = ResourceActivityOperationId::new(),
            2 => row.recorded_by.email = "another@example.com".into(),
            3 => row.recorded_at += time::Duration::seconds(1),
            4 => {
                row.selection.act = None;
                row.sources.act = None;
            }
            5 => {
                let head = &f.material.resource_head;
                row.selection.resource = ResourceCaptureRef {
                    id: head.id,
                    revision: head.revision,
                    capture_digest: head.receipt.capture_digest,
                };
                row.sources.resource = head.clone();
            }
            _ => row.recorded_by.id = domain::identity::UserId::new(),
        }
        rehash(row);
        resource_activity_receipt_matches(hearing_support::hasher().as_ref(), row)
            .unwrap_or_else(|error| panic!("valid standalone association {fault}: {error}"));
        assert!(
            resource_hearing_creation_matches(hearing_support::hasher().as_ref(), &changed)
                .is_err(),
            "fault {fault}"
        );
    }
}

#[test]
fn corrupt_target_capture_is_rejected_without_requiring_a_creation_cycle() {
    let result = creation(&Fixture::new(Role::Owner));
    let mut row = result.association;
    let ResourceActivityTargetDetail::ResourceHearing(hearing) = &mut row.sources.target else {
        panic!("resource hearing target expected");
    };
    hearing.review.support.digest = Sha256Digest::from_array([8; 32]);
    assert!(resource_hearing_receipt_matches(hearing_support::hasher().as_ref(), hearing).is_err());
    assert!(resource_activity_receipt_matches(hearing_support::hasher().as_ref(), &row).is_err());
}

#[test]
fn organizational_unlink_preserves_creation_and_cannot_replace_its_initial_link() {
    let f = Fixture::new(Role::Owner);
    let initial = creation(&f);
    let mut activity = resource_activity_support::Fixture {
        case_id: f.case(),
        resource_id: f.resource(),
        command: resource_activity_command_from_detail(&initial.association).unwrap(),
        material: ResourceActivityMaterial {
            case_id: f.case(),
            base: None,
            administration: f.material.administration.clone(),
            resource_head: f.material.resource_head.clone(),
            sources: initial.association.sources.clone(),
        },
    };
    activity.unlink(initial.association.clone());
    let unlinked = activity.committed(&f.actor);
    assert_eq!(unlinked.status, ResourceActivityStatus::Unlinked);
    assert_eq!(unlinked.selection, initial.association.selection);
    assert_eq!(unlinked.sources, initial.association.sources);
    resource_hearing_creation_matches(hearing_support::hasher().as_ref(), &initial).unwrap();
    let mut replaced = initial;
    replaced.association = unlinked;
    assert!(
        resource_hearing_creation_matches(hearing_support::hasher().as_ref(), &replaced).is_err()
    );
}

#[test]
fn current_resource_hearing_target_is_verified_in_target_scoped_reads() {
    let f = Fixture::new(Role::Owner);
    let result = creation(&f);
    let target = ResourceActivityTargetId::ResourceHearing(f.command.hearing_id);
    let query = ResourceActivityTargetQuery::new(10, None, None).unwrap();
    let expected = ResourceActivityTargetPage {
        checked_at: case_support::instant(),
        associations: vec![ResourceActivityView {
            association: result.association,
            checked_at: case_support::instant(),
            current_target: ResourceActivityCurrentTarget::ResourceHearing(Box::new(
                result.hearing,
            )),
        }],
        has_more: false,
        next_after_id: None,
    };
    let returned = expected.clone();
    let mut store = resource_activity_support::MockStore::new();
    store
        .expect_list_for_target()
        .times(1)
        .return_once(move |_, _, actual, _, _| {
            assert_eq!(actual, target);
            Ok(returned)
        });
    let mut reader = f.actor;
    reader.role = Role::Paralegal;
    let (identity, _) = procedural_resource_support::identity_for(reader, 2);
    assert_eq!(
        resource_activity_support::service(store, identity)
            .list_for_target("session", f.material.case_id, target, query)
            .unwrap(),
        expected
    );
}

#[test]
fn public_canonical_bytes_reproduce_review_and_capture_without_association_state() {
    let result = creation(&Fixture::new(Role::Owner));
    let h = &result.hearing;
    let hasher = hearing_support::hasher();
    let submitted = resource_hearing_submission_bytes(hasher.as_ref(), &h.review).unwrap();
    assert!(submitted.starts_with(b"RHPR1"));
    assert_eq!(hasher.hash_bytes(&submitted), h.review.submission_digest);
    let mut expected = b"RHCR1".to_vec();
    expected.extend_from_slice(h.review.submission_digest.as_bytes());
    expected.extend_from_slice(&1_u32.to_be_bytes());
    expected.extend_from_slice(&h.recorded_at.unix_timestamp().to_be_bytes());
    expected.extend_from_slice(&h.recorded_at.nanosecond().to_be_bytes());
    expected.extend_from_slice(&0_i32.to_be_bytes());
    expected.extend_from_slice(&0_u32.to_be_bytes());
    assert_eq!(resource_hearing_capture_bytes(h), expected);
    assert_eq!(hasher.hash_bytes(&expected), h.capture_digest);
    let mut later = h.clone();
    later.recorded_at += time::Duration::nanoseconds(1);
    assert_eq!(
        resource_hearing_submission_bytes(hasher.as_ref(), &later.review).unwrap(),
        submitted
    );
    assert_ne!(resource_hearing_capture_bytes(&later), expected);
}
