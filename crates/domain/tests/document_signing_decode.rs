use super::{binding, consent};
use domain::document_signing::{DocumentConsent, SigningBindingStatement};

#[test]
fn exact_canonical_bytes_round_trip_without_deserializing_unchecked_fields() {
    let binding = binding();
    let consent = consent();
    assert_eq!(
        SigningBindingStatement::from_canonical_bytes(&binding.canonical_bytes()).unwrap(),
        binding
    );
    assert_eq!(
        DocumentConsent::from_canonical_bytes(&consent.canonical_bytes()).unwrap(),
        consent
    );
}

#[test]
fn changed_format_purpose_lengths_and_nil_references_are_rejected() {
    let binding = binding().canonical_bytes();
    let consent = consent().canonical_bytes();
    for position in [0, 8, 9] {
        let mut invalid = binding;
        invalid[position] ^= 1;
        assert!(SigningBindingStatement::from_canonical_bytes(&invalid).is_err());
        let mut invalid = consent;
        invalid[position] ^= 1;
        assert!(DocumentConsent::from_canonical_bytes(&invalid).is_err());
    }
    for range in [10..26, 62..78, 94..110] {
        let mut invalid = binding;
        invalid[range.clone()].fill(0);
        assert!(SigningBindingStatement::from_canonical_bytes(&invalid).is_err());
        let mut invalid = consent;
        invalid[range].fill(0);
        assert!(DocumentConsent::from_canonical_bytes(&invalid).is_err());
    }
    for range in [142..158, 158..174, 174..190, 190..194] {
        let mut invalid = consent;
        invalid[range].fill(0);
        assert!(DocumentConsent::from_canonical_bytes(&invalid).is_err());
    }
    for bytes in [&binding[..141], &[binding.as_slice(), &[0]].concat()] {
        assert!(SigningBindingStatement::from_canonical_bytes(bytes).is_err());
    }
    for bytes in [&consent[..257], &[consent.as_slice(), &[0]].concat()] {
        assert!(DocumentConsent::from_canonical_bytes(bytes).is_err());
    }
    assert!(SigningBindingStatement::from_canonical_bytes(&consent).is_err());
    assert!(DocumentConsent::from_canonical_bytes(&binding).is_err());
}

#[test]
fn decoders_reapply_account_and_trust_counter_invariants() {
    for range in [58..62, 78..86, 86..94] {
        let mut invalid = binding().canonical_bytes();
        invalid[range.clone()].fill(if range.start == 58 { 0 } else { 255 });
        assert!(SigningBindingStatement::from_canonical_bytes(&invalid).is_err());
        let mut invalid = consent().canonical_bytes();
        invalid[range.clone()].fill(if range.start == 58 { 0 } else { 255 });
        assert!(DocumentConsent::from_canonical_bytes(&invalid).is_err());
    }
}
