#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_tracked_support;
mod deadline_tracking_storage_support;

use application::deadlines::decode_deadline_tracking_capture;
use deadline_tracking_storage_support::*;

#[test]
fn every_truncated_capture_is_rejected_including_partial_hashes_and_revision() {
    let mut recorded_value = capture();
    recorded(&mut recorded_value, 1);
    let frames = [
        frame(&[2, 2, 0, 1, 0], &capture()),
        frame(&[2, 2, 0, 1, 0], &recorded_value),
        frame(&[2, 1, 0, 2, 3, 0, 1, 1, 2, 2, 3], &pending_mixed()),
        frame(&[0, 0, 0, 0, 0], &legacy()),
    ];
    for bytes in frames {
        assert!(decode_deadline_tracking_capture(&bytes).is_ok());
        for end in 0..bytes.len() {
            assert!(
                decode_deadline_tracking_capture(&bytes[..end]).is_err(),
                "length {end}"
            );
        }
    }
}

#[test]
fn decoder_rejects_trailing_bytes_and_frames_larger_than_the_122_byte_bound() {
    let bytes = frame(&[2, 2, 0, 1, 0], &capture());
    for tail in [vec![0], vec![255], vec![0; 21], bytes.clone()] {
        let mut altered = bytes.clone();
        altered.extend(tail);
        assert!(decode_deadline_tracking_capture(&altered).is_err());
    }
    assert!(decode_deadline_tracking_capture(&[0; 123]).is_err());
    assert!(decode_deadline_tracking_capture(&vec![0; 1024]).is_err());
}

#[test]
fn storage_suffix_does_not_accept_a_version_prefix_or_a_whole_state_frame() {
    let bytes = frame(&[2, 2, 0, 1, 0], &capture());
    for prefix in [b"DLST1", b"DLST2", b"DLOB1", b"DLRV2"] {
        let mut altered = prefix.to_vec();
        altered.extend(&bytes);
        assert!(decode_deadline_tracking_capture(&altered).is_err());
    }
    let detail = deadline_tracked_support::accepted();
    let state =
        application::deadlines::deadline_capture_bytes(inputs::hasher().as_ref(), &detail).unwrap();
    assert!(decode_deadline_tracking_capture(&state).is_err());
}

#[test]
fn policy_and_state_discriminants_are_closed_sets() {
    let bytes = frame(&[2, 2, 0, 1, 0], &capture());
    for position in 0..4 {
        for tag in 3..=255 {
            let mut altered = bytes.clone();
            altered[position] = tag;
            assert!(
                decode_deadline_tracking_capture(&altered).is_err(),
                "position {position}, tag {tag}"
            );
        }
    }
}

#[test]
fn reason_count_is_bounded_and_must_match_the_review_state() {
    for header in [
        vec![2, 2, 0, 2, 0],
        vec![2, 2, 0, 1, 1, 1, 0],
        vec![0, 0, 0, 0, 1, 1, 3],
        vec![2, 2, 0, 2, 9],
        vec![2, 2, 0, 2, 255],
    ] {
        assert!(decode_deadline_tracking_capture(&frame(&header, &capture())).is_err());
    }
    let mut too_many = vec![0, 0, 0, 2, 9];
    too_many.extend([0, 1, 0, 2, 0, 3, 1, 0, 1, 2, 1, 3, 2, 2, 2, 3, 2, 3]);
    assert!(decode_deadline_tracking_capture(&frame(&too_many, &capture())).is_err());
}

#[test]
fn reason_dependencies_and_tags_are_strictly_typed() {
    for (dependency, reason) in [
        (3, 0),
        (255, 0),
        (1, 4),
        (1, 255),
        (0, 0),
        (1, 1),
        (2, 0),
        (2, 1),
    ] {
        let bytes = frame(&[2, 2, 0, 2, 1, dependency, reason], &capture());
        assert!(
            decode_deadline_tracking_capture(&bytes).is_err(),
            "pair {dependency},{reason}"
        );
    }
}

#[test]
fn decoder_never_sorts_or_deduplicates_review_reasons() {
    for pairs in [[0, 1, 0, 1], [1, 0, 0, 1], [0, 3, 0, 2], [2, 3, 1, 3]] {
        let mut header = vec![0, 0, 0, 2, 2];
        header.extend(pairs);
        assert!(decode_deadline_tracking_capture(&frame(&header, &capture())).is_err());
    }
}

#[test]
fn administrative_presence_and_revision_have_no_implicit_defaults() {
    let bytes = frame(&[2, 2, 0, 1, 0], &capture());
    for flag in 2..=255 {
        let mut altered = bytes.clone();
        altered[37] = flag;
        assert!(
            decode_deadline_tracking_capture(&altered).is_err(),
            "flag {flag}"
        );
    }
    let mut value = capture();
    recorded(&mut value, 1);
    let recorded_bytes = frame(&[2, 2, 0, 1, 0], &value);
    let mut zero = recorded_bytes.clone();
    zero[38..42].fill(0);
    assert!(decode_deadline_tracking_capture(&zero).is_err());
    let mut undeclared_revision = recorded_bytes.clone();
    undeclared_revision[37] = 0;
    assert!(decode_deadline_tracking_capture(&undeclared_revision).is_err());
    let mut missing_revision = recorded_bytes;
    missing_revision.drain(38..42);
    assert!(decode_deadline_tracking_capture(&missing_revision).is_err());
}
