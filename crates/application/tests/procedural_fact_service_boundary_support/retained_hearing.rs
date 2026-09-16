use super::inconsistent;
use crate::{hearing_source_support as source, procedural_fact_service_support::*, store::*};
use application::{hearing_results::*, procedural_facts::*};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use std::sync::{atomic::Ordering, Arc};

fn rehash(value: &mut HearingResultSnapshot) {
    value.values_digest = hasher().hash_bytes(&value.values.canonical_bytes());
    value.receipt.submission_digest = hasher().hash_bytes(&source::receipt_bytes(value));
    hearing_result_snapshot_receipt_matches(hasher().as_ref(), value).unwrap();
}
fn selection(value: &HearingResultSnapshot, index: usize) -> ResolutionValues {
    let base = values();
    ResolutionValues::new(ResolutionValuesInput {
        class: base.class().clone(),
        subtype: None,
        issuer: base.issuer().clone(),
        issued_at: base.issued_at(),
        summary: base.summary().clone(),
        provenance: FactProvenance::HearingResult {
            reference: FactHearingRef {
                hearing_id: value.hearing_id,
                result_id: value.id,
                revision: value.revision,
                agreement_id: Some(value.values.agreements()[index].id()),
            },
            locator: FactLabel::new("Declared agreement").unwrap(),
            support: None,
        },
    })
}
fn base(actor: UserId, case: CaseId, hearing: &HearingResultSnapshot) -> FactDetail {
    let values = selection(hearing, 0);
    let selected = FactSourceSelection::from_values(&ProceduralFactValues::Resolution(Box::new(
        values.clone(),
    )));
    let projection = resolve_fact_hearings(
        hasher().as_ref(),
        case,
        &selected,
        std::slice::from_ref(hearing),
    )
    .unwrap()
    .pop()
    .unwrap();
    let mut sources = empty();
    sources.resolved.hearing_results.push(projection.snapshot);
    sources.views.hearing_results.push(projection.view);
    let result = detail(
        actor,
        case,
        &record_command(values.clone()),
        values,
        sources,
    );
    fact_receipt_matches(hasher().as_ref(), &result).unwrap();
    result
}
fn prepare_changed_agreement(substitute: bool) -> Result<FactDraft, application::ApplicationError> {
    let actor = application::identity::Principal {
        id: UserId::new(),
        email: "actor@example.test".into(),
        role: Role::Owner,
    };
    let mut identity = MockIdentity::new();
    let principal = actor.clone();
    identity
        .expect_authenticate()
        .times(1..=2)
        .returning(move |_| Ok(principal.clone()));
    let mut hearing = source::source(5, 6, 1, false, &[0, 1]);
    rehash(&mut hearing);
    let case = hearing.case_id;
    let original = base(actor.id, case, &hearing);
    let FactTarget::Resolution(id) = original.snapshot.target() else {
        unreachable!()
    };
    let values = selection(&hearing, 1);
    let command = ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::new(),
        id,
        FactChange::correct(
            FactRevision::initial(),
            values,
            text("Select other agreement"),
        ),
    ));
    if substitute {
        let mut input = source::values_input(&hearing.values);
        input.summary = HearingResultText::new("Substituted common source summary").unwrap();
        hearing.values = HearingResultValues::new(input).unwrap();
        rehash(&mut hearing);
        assert_ne!(
            original.sources.resolved.hearing_results[0].values_digest,
            hearing.values_digest
        );
    }
    let mut prep = preparation(case);
    prep.base = Some(original);
    prep.source_material.hearing_results.push(hearing);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator::default());
    let (service, _) = service(store, identity, validator.clone());
    let result = service.prepare("session", case, command);
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
    result
}
#[test]
fn selecting_another_agreement_from_the_same_exact_result_is_allowed() {
    let result = prepare_changed_agreement(false).unwrap();
    assert_eq!(result.sources.resolved.hearing_results.len(), 1);
    assert_eq!(
        result.sources.views.hearing_results[0]
            .agreement
            .as_ref()
            .unwrap()
            .text()
            .as_str(),
        "Agreement 1 in revision 1"
    );
}
#[test]
fn changed_agreement_cannot_replace_the_common_exact_result_projection() {
    inconsistent(prepare_changed_agreement(true));
}
