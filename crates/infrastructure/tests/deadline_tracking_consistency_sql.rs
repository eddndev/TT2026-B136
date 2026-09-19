mod case_administration_support;
mod deadline_observations_sql_support;
mod deadline_tracking_sql_support;

use application::deadline_reevaluation::*;
use deadline_observations_sql_support as observations;
use deadline_tracking_sql_support::*;
use uuid::Uuid;

#[test]
fn two_absent_payloads_are_legacy_but_one_absent_payload_is_inconsistent() {
    let Some(mut db) = fixture() else { return };
    let (tracking, manifest) = pair(&observations::profile(), [0; 3], 0, &[]);
    assert!(consistent(&mut db, None, None));
    assert!(!consistent(&mut db, Some(&tracking), None));
    assert!(!consistent(&mut db, None, Some(&manifest)));
    assert!(consistent(&mut db, Some(&tracking), Some(&manifest)));
}

#[test]
fn profile_only_payloads_accept_global_private_and_explicit_unknown_optional_dependencies() {
    let Some(mut db) = fixture() else { return };
    for private in [false, true] {
        let mut value = observations::profile();
        if private {
            value.entries[0].case_id = Some(value.case_id);
        }
        for (policies, state, reasons) in [
            ([0; 3], 0, vec![]),
            ([2, 0, 0], 1, vec![]),
            ([1, 0, 0], 1, vec![]),
            ([0; 3], 2, vec![[0, 3]]),
        ] {
            let (tracking, manifest) = pair(&value, policies, state, &reasons);
            assert!(consistent(&mut db, Some(&tracking), Some(&manifest)));
        }
    }
}

#[test]
fn accepted_payloads_cover_resolution_notification_result_and_calendar_shapes() {
    let Some(mut db) = fixture() else { return };
    for family in [
        DependencyFamily::Resolution,
        DependencyFamily::HearingResult,
    ] {
        let mut value = observations::profile();
        let mut source = observations::entry(ObservationRole::Source, family, 3);
        source.case_id = Some(value.case_id);
        if family == DependencyFamily::HearingResult {
            source.hearing_id = Some(Uuid::nil());
        }
        value.entries.push(source);
        for policy in [1, 2] {
            let (tracking, manifest) = pair(&value, [2, policy, 0], 1, &[]);
            assert!(consistent(&mut db, Some(&tracking), Some(&manifest)));
        }
    }
    for policy in [1, 2] {
        let (tracking, manifest) = pair(&observations::notification(), [2, policy, policy], 1, &[]);
        assert!(consistent(&mut db, Some(&tracking), Some(&manifest)));
    }
    let mut value = observations::profile();
    value.entries.push(observations::entry(
        ObservationRole::Calendar,
        DependencyFamily::Calendar,
        5,
    ));
    let (tracking, manifest) = pair(&value, [2, 0, 2], 1, &[]);
    assert!(consistent(&mut db, Some(&tracking), Some(&manifest)));
}

#[test]
fn absent_source_or_calendar_cannot_carry_a_policy_or_review_requirement() {
    let Some(mut db) = fixture() else { return };
    for (policies, state, reasons) in [
        ([1, 1, 0], 1, vec![]),
        ([1, 0, 2], 1, vec![]),
        ([1, 0, 0], 2, vec![[1, 3]]),
        ([1, 0, 0], 2, vec![[2, 3]]),
        ([1, 1, 0], 2, vec![[1, 2]]),
        ([1, 0, 1], 2, vec![[2, 2]]),
    ] {
        let (tracking, manifest) = pair(&observations::profile(), policies, state, &reasons);
        parse(&mut db, &tracking);
        assert!(!consistent(&mut db, Some(&tracking), Some(&manifest)));
    }
}

#[test]
fn present_undeclared_dependencies_require_pending_policy_reasons_for_each_scope() {
    let Some(mut db) = fixture() else { return };
    let value = observations::notification();
    for policies in [[1, 0, 1], [1, 1, 0], [1, 0, 0]] {
        let (tracking, manifest) = pair(&value, policies, 1, &[]);
        parse(&mut db, &tracking);
        assert!(!consistent(&mut db, Some(&tracking), Some(&manifest)));
    }
    for reasons in [vec![[0, 1]], vec![[0, 1], [1, 3]], vec![[0, 1], [2, 3]]] {
        let (tracking, manifest) = pair(&value, [2, 0, 0], 2, &reasons);
        parse(&mut db, &tracking);
        assert!(!consistent(&mut db, Some(&tracking), Some(&manifest)));
    }
    let (tracking, manifest) = pair(&value, [2, 0, 0], 2, &[[0, 1], [1, 3], [2, 3]]);
    assert!(consistent(&mut db, Some(&tracking), Some(&manifest)));
}

#[test]
fn accepted_notification_requires_observed_parent_without_rejecting_legacy_or_pending() {
    let Some(mut db) = fixture() else { return };
    let mut value = observations::notification();
    value.entries.pop();
    for policy in [1, 2] {
        let (tracking, manifest) = pair(&value, [1, policy, 1], 1, &[]);
        observations::parse(&mut db, &manifest);
        parse(&mut db, &tracking);
        assert!(!consistent(&mut db, Some(&tracking), Some(&manifest)));
    }
    for (policies, state, reasons) in [
        ([0; 3], 0, vec![]),
        ([1, 2, 1], 2, vec![[1, 0]]),
        ([1, 1, 1], 2, vec![[1, 2]]),
    ] {
        let (tracking, manifest) = pair(&value, policies, state, &reasons);
        assert!(consistent(&mut db, Some(&tracking), Some(&manifest)));
    }
}

#[test]
fn retirement_review_is_valid_for_fixed_policies_without_inventing_observed_status() {
    let Some(mut db) = fixture() else { return };
    let value = observations::notification();
    let (tracking, manifest) = pair(&value, [1; 3], 2, &[[0, 2], [1, 2], [2, 2]]);
    assert!(consistent(&mut db, Some(&tracking), Some(&manifest)));
    let (tracking, manifest) = pair(
        &value,
        [0; 3],
        2,
        &[[0, 2], [0, 3], [1, 2], [1, 3], [2, 2], [2, 3]],
    );
    assert!(consistent(&mut db, Some(&tracking), Some(&manifest)));
}

#[test]
fn digest_comparison_binds_all_manifest_bytes_not_only_role_presence() {
    let Some(mut db) = fixture() else { return };
    let value = observations::profile();
    let (mut tracking, manifest) = pair(&value, [1, 0, 0], 1, &[]);
    assert!(consistent(&mut db, Some(&tracking), Some(&manifest)));
    tracking[5] ^= 1;
    parse(&mut db, &tracking);
    assert!(!consistent(&mut db, Some(&tracking), Some(&manifest)));
    let (tracking, _) = pair(&value, [1, 0, 0], 1, &[]);
    for change in 0..4 {
        let mut different = value.clone();
        match change {
            0 => different.entries[0].id = Uuid::nil(),
            1 => different.entries[0].revision += 1,
            2 => {
                different.entries[0].submission_digest =
                    domain::crypto::Sha256Digest::from_array([9; 32])
            }
            _ => {
                different.entries[0].evidence_digest =
                    domain::crypto::Sha256Digest::from_array([9; 32])
            }
        }
        let (_, manifest) = pair(&different, [1, 0, 0], 1, &[]);
        observations::parse(&mut db, &manifest);
        assert!(!consistent(&mut db, Some(&tracking), Some(&manifest)));
    }
}

#[test]
fn invalid_bytes_raise_check_violation_even_when_digest_would_also_disagree() {
    let Some(mut db) = fixture() else { return };
    let (tracking, manifest) = pair(&observations::profile(), [1, 0, 0], 1, &[]);
    assert!(consistent(&mut db, Some(&tracking), Some(&manifest)));
    for end in [0, 4, tracking.len() - 1] {
        inconsistent_bytes(&mut db, &tracking[..end], &manifest);
    }
    for end in [0, 5, manifest.len() - 1] {
        inconsistent_bytes(&mut db, &tracking, &manifest[..end]);
    }
    let mut malformed = tracking.clone();
    malformed[3] = 255;
    inconsistent_bytes(&mut db, &malformed, &manifest);
    let mut malformed = manifest.clone();
    malformed[4] = b'2';
    inconsistent_bytes(&mut db, &tracking, &malformed);
}
