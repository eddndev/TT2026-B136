#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
mod participant_support;
use application::participants::*;
use domain::{cases::CaseId, identity::Role, typed_participants::ParticipantKind};
use participant_support::*;

#[test]
fn typed_filter_is_part_of_the_query_before_pagination() {
    let query = ParticipantQuery::new(1, None, None, None, ParticipantStatusFilter::All)
        .unwrap()
        .with_profile_filter(
            Some(ParticipantKind::ControlJudge),
            ParticipantProfileFilter::Typed,
        );
    assert_eq!(query.kind(), Some(ParticipantKind::ControlJudge));
    assert_eq!(query.profile(), ParticipantProfileFilter::Typed);
}
#[test]
fn exact_revision_uses_its_port_and_retains_original_actor() {
    let (identity, actor) = identity(Role::Paralegal, 1);
    let case = CaseId::new();
    let id = ParticipantId::new();
    let expected = ParticipantDetail::from(snapshot(case, id, actor.id, 2));
    let result = expected.clone();
    let mut store = MockStore::new();
    store
        .expect_get_revision()
        .times(1)
        .return_once(move |_, _, _, revision, _| {
            assert_eq!(revision.get(), 2);
            Ok(result)
        });
    let (service, _) = service(store, identity);
    assert_eq!(
        service
            .get_revision("session", case, id, ParticipantRevision::new(2).unwrap())
            .unwrap(),
        expected
    );
}
