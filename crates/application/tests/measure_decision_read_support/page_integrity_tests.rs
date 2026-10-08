use super::*;
use domain::crypto::DocumentHasher;

fn list_items(
    items: Vec<MeasureDecisionStoredOperation>,
) -> Result<MeasureDecisionPage, ApplicationError> {
    let case_id = items[0].group.review.case_id;
    let returned = page(case_id, items);
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(returned));
    service(store, identity(&reader(Role::Paralegal))).list(
        "session",
        case_id,
        MeasureDecisionReadQuery::default(),
    )
}

#[test]
fn page_rejects_distinct_root_decisions_claiming_one_group_operation() {
    let first = operation(10);
    let mut fixture = root_fixture(20);
    fixture.command.operation_id = first.origin.operation_id;
    let second = stored(fixture.capture(), empty_history());
    assert!(list_items(vec![first, second]).is_err());
}

#[test]
fn page_rejects_distinct_ancestor_groups_claiming_one_decision_identity() {
    let first = operation(100);
    let mut fixture = root_fixture(101);
    fixture.command.decision_id = first.origin.decision_id;
    let second = stored(fixture.capture(), empty_history());
    assert_ne!(
        first.group.measures[0].result.id,
        second.group.measures[0].result.id
    );
    assert!(list_items(vec![dependent(&first, 10), dependent(&second, 20)]).is_err());
}

#[test]
fn page_rejects_one_measure_revision_owned_by_distinct_ancestor_groups() {
    let first = operation(100);
    let mut fixture = root_fixture(101);
    let measure = first.group.measures[0].result.id;
    let mut effects = fixture.command.outcome.changes().unwrap().to_vec();
    let MeasureEffect::Impose(proposal) = &mut effects[0] else {
        unreachable!()
    };
    proposal.id = measure;
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
    fixture.material.result_sources[0].id = measure;
    let second = stored(fixture.capture(), empty_history());
    assert_ne!(first.origin.decision_id, second.origin.decision_id);
    assert!(list_items(vec![dependent(&first, 10), dependent(&second, 20)]).is_err());
}

#[test]
fn an_exact_group_may_be_both_a_page_item_and_another_items_ancestor() {
    let first = operation(100);
    let second = dependent(&first, 101);
    let expected = vec![first, second];
    assert_eq!(list_items(expected.clone()).unwrap().items, expected);
}

#[test]
fn individually_valid_page_groups_cannot_disagree_on_shared_sources() {
    for mutation in 0..4 {
        let first = operation(10);
        let mut fixture = root_fixture(20);
        let sources = &mut fixture.material.result_sources[0].sources;
        match mutation {
            0 => sources.subject.changed_by.email = "different subject recorder".into(),
            1 => {
                sources
                    .supervisor
                    .as_mut()
                    .unwrap()
                    .bound_subject
                    .as_mut()
                    .unwrap()
                    .changed_by
                    .email = "different bound subject recorder".into()
            }
            2 => {
                crate::participant_support::typed_mut(sources.supervisor.as_mut().unwrap())
                    .changed_by
                    .email = "different participant recorder".into()
            }
            _ => fixture.material.support.name = "other-resolution-name.pdf".into(),
        }
        let second = stored(fixture.capture(), empty_history());
        assert!(list_items(vec![first, second]).is_err());
    }
}

fn anchored(
    serial: u128,
    hearing: application::precautionary_hearings::PrecautionaryHearingCapture,
) -> MeasureDecisionStoredOperation {
    let mut fixture = root_fixture(serial);
    fixture.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: hearing.review.command.hearing_id,
        revision: hearing.review.result_revision,
        capture_digest: hearing.capture_digest,
    });
    fixture.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        hearing,
    )));
    let group = fixture
        .prepare()
        .unwrap()
        .into_group_capture(&Hasher, at() + Duration::seconds(2))
        .unwrap();
    stored(group, empty_history())
}

#[test]
fn page_compares_full_repeated_hearing_captures_instead_of_only_review_digests() {
    let first = crate::precautionary_receipt_support::scheduled();
    let mut second = first.clone();
    second.recorded_at += Duration::seconds(1);
    second.capture_digest = Hasher.hash_bytes(
        &application::precautionary_hearings::precautionary_hearing_capture_bytes(&second).unwrap(),
    );
    assert_eq!(first.review, second.review);
    assert!(list_items(vec![anchored(10, first), anchored(20, second)]).is_err());
}

#[test]
fn distinct_decisions_may_retain_the_same_exact_full_hearing_capture() {
    let hearing = crate::precautionary_receipt_support::scheduled();
    let expected = vec![anchored(10, hearing.clone()), anchored(20, hearing)];
    assert_eq!(list_items(expected.clone()).unwrap().items, expected);
}
