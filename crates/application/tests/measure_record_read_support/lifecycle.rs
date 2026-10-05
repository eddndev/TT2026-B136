use super::*;

#[test]
fn every_mixed_record_family_retains_its_real_owner_full_history_and_exact_selection() {
    for original in mixed(10) {
        let actor = reader(Role::Paralegal);
        for kind in READS {
            let store = successful_store(&actor, &original, original.clone(), kind);
            assert_eq!(
                read(&service(store, identity(&actor)), kind, &original).unwrap(),
                vec![original.clone()]
            );
        }
    }
}

#[test]
fn actual_marked_head_is_returned_without_falling_back_to_an_older_valid_record() {
    let valid = mixed(10).pop().unwrap();
    let marked = administrative(&valid, 999, true);
    let OwnedMeasureRecord::Administrative { capture, .. } = &marked.record else {
        unreachable!()
    };
    assert_eq!(
        capture.result.validity,
        MeasureCaptureValidity::EnteredInError
    );
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let store = successful_store(&actor, &marked, marked.clone(), kind);
        assert_eq!(
            read(&service(store, identity(&actor)), kind, &marked).unwrap(),
            vec![marked.clone()]
        );
    }
    let store = successful_store(&actor, &valid, valid.clone(), ReadKind::Exact);
    assert_eq!(
        read(&service(store, identity(&actor)), ReadKind::Exact, &valid).unwrap(),
        vec![valid]
    );
}

#[test]
fn exact_terminal_declaration_is_readable_without_legal_status_inference() {
    let first = root_fixture(10).capture();
    let mut later = crate::effect_support::LaterFixture::confirm(&first);
    later.effects(vec![MeasureEffect::Revoke {
        previous: reference(&first.measures[0]),
    }]);
    let history = later.evidence.clone();
    let original = from_group(&later.capture(), &history);
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let store = successful_store(&actor, &original, original.clone(), kind);
        assert_eq!(
            read(&service(store, identity(&actor)), kind, &original).unwrap(),
            vec![original.clone()]
        );
    }
}
