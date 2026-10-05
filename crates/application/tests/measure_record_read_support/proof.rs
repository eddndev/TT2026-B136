use super::*;

#[test]
fn detail_reference_case_and_complete_record_must_match_its_validated_owner() {
    for original in mixed(10) {
        for mutation in 0..4 {
            let mut returned = original.clone();
            match mutation {
                0 => returned.case_id = crate::participant_support::other_case(),
                1 => {
                    returned.reference = PrecautionaryMeasureRef::new(
                        original.reference.id(),
                        original.reference.revision(),
                        Sha256Digest::from_array([99; 32]),
                    )
                }
                _ => match &mut returned.record {
                    OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V1(m)) => {
                        if mutation == 2 {
                            m.owner.group_digest = Sha256Digest::from_array([99; 32]);
                        } else {
                            m.capture.actor.email = "invented@example.test".into();
                        }
                    }
                    OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V2(m)) => {
                        if mutation == 2 {
                            m.owner.group_digest = Sha256Digest::from_array([99; 32]);
                        } else {
                            m.capture.actor.email = "invented@example.test".into();
                        }
                    }
                    OwnedMeasureRecord::Administrative { owner, capture } => {
                        if mutation == 2 {
                            owner.capture_digest = Sha256Digest::from_array([99; 32]);
                        } else {
                            capture.actor.email = "invented@example.test".into();
                        }
                    }
                },
            }
            reject_reads(&original, &returned);
        }
    }
}

#[test]
fn exact_proof_requires_selected_owner_every_ancestor_and_all_original_commitments() {
    let original = mixed(10).pop().unwrap();
    for mutation in 0..9 {
        let mut returned = original.clone();
        let evidence = &mut returned.record_history;
        match mutation {
            0 => evidence.records.judicial.groups.clear(),
            1 => {
                evidence.records.administrative.pop();
            }
            2 => {
                evidence.records.administrative.remove(0);
            }
            3 => evidence.decisions.clear(),
            4 => {
                evidence.records.judicial.groups[0].origin.group_digest =
                    Sha256Digest::from_array([99; 32])
            }
            5 => {
                evidence.records.administrative[0].origin.capture_digest =
                    Sha256Digest::from_array([99; 32])
            }
            6 => evidence.decisions[0].origin.group_digest = Sha256Digest::from_array([99; 32]),
            7 => evidence
                .records
                .administrative
                .push(evidence.records.administrative[0].clone()),
            _ => evidence
                .records
                .judicial
                .groups
                .extend(initial(20).record_history.records.judicial.groups),
        }
        reject_reads(&original, &returned);
    }
}

#[test]
fn evidence_transport_order_does_not_rewrite_returned_original_history() {
    let original = mixed(10).pop().unwrap();
    let mut reordered = original.clone();
    reordered.record_history.records.administrative.reverse();
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let store = successful_store(&actor, &original, reordered.clone(), kind);
        assert_eq!(
            read(&service(store, identity(&actor)), kind, &original).unwrap(),
            vec![reordered.clone()]
        );
    }
}

#[test]
fn unselected_siblings_are_required_as_part_of_the_actual_owner() {
    let group = crate::measure_decision_fixtures::Fixture::multiple().capture();
    let original = from_group(&group, &crate::effect_support::empty_history());
    let mut returned = original.clone();
    returned.record_history.records.judicial.groups[0]
        .capture
        .measures
        .pop();
    reject_reads(&original, &returned);
}
