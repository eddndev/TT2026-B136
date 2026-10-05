use super::*;

#[test]
fn semantic_arrays_must_keep_normalized_order_and_disjoint_identities() {
    let stored = stored_outcome(mixed());
    for path in [
        "/effects",
        "/effects/5/predecessors",
        "/effects/5/successors",
    ] {
        stored.rejects(|view| {
            view.pointer_mut(path)
                .unwrap()
                .as_array_mut()
                .unwrap()
                .reverse()
        });
    }
    stored.rejects(|view| {
        view["effects"][1]["previous"]["id"] = view["effects"][0]["proposal"]["id"].clone()
    });
    stored.rejects(|view| {
        view["effects"][5]["predecessors"][1] = view["effects"][5]["predecessors"][0].clone()
    });
    stored.rejects(|view| {
        view["effects"][5]["successors"][0]["id"] =
            view["effects"][5]["predecessors"][0]["id"].clone()
    });
}

#[test]
fn empty_and_oversized_effect_or_substitution_arrays_are_rejected() {
    let stored = stored_outcome(mixed());
    for path in [
        "/effects",
        "/effects/5/predecessors",
        "/effects/5/successors",
    ] {
        for count in [0, 33, 65_536] {
            stored
                .rejects(|view| *view.pointer_mut(path).unwrap() = json!(vec![Value::Null; count]));
        }
    }
}

#[test]
fn individually_bounded_substitution_sides_cannot_exceed_32_affected_identities() {
    let predecessors: Vec<_> = (1..=16).map(reference).collect();
    let successors: Vec<_> = (17..=32).map(proposal).collect();
    let original = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Substitute {
            predecessors: predecessors.clone(),
            successors: successors.clone(),
        },
    ]))
    .unwrap();
    let mut projection = outcome_view(&original);
    assert!(outcome(&original.canonical_bytes(), &projection).is_ok());
    let mut successors = successors;
    successors.push(proposal(33));
    let extra = outcome_view(
        &MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Impose(proposal(33)),
        ]))
        .unwrap(),
    );
    projection["effects"][0]["successors"]
        .as_array_mut()
        .unwrap()
        .push(extra["effects"][0]["proposal"].clone());
    let mut bytes = b"MEFX1\x00\x00\x00\x00\x01\x05".to_vec();
    bytes.extend_from_slice(&16u32.to_be_bytes());
    for reference in predecessors {
        bytes.extend_from_slice(reference.id().as_uuid().as_bytes());
        bytes.extend_from_slice(&reference.revision().get().to_be_bytes());
        bytes.extend_from_slice(reference.digest().as_bytes());
    }
    bytes.extend_from_slice(&17u32.to_be_bytes());
    for proposal in successors {
        bytes.extend_from_slice(proposal.id.as_uuid().as_bytes());
        let values = proposal.values.canonical_bytes();
        bytes.extend_from_slice(&(values.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&values);
    }
    stored_outcome(original).rejects_bytes(&bytes, &projection);
}
