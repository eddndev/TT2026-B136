use super::generate_with;
use std::cell::Cell;
use zeroize::Zeroizing;

#[test]
fn one_entropy_call_fills_the_entire_32_byte_owned_token() {
    let calls = Cell::new(0);
    let token: Zeroizing<[u8; 32]> = generate_with(|destination| {
        calls.set(calls.get() + 1);
        assert_eq!(destination.len(), 32);
        destination.fill(0x5a);
        Ok(())
    })
    .unwrap_or_else(|_| panic!("injected entropy success was rejected"));
    assert_eq!(calls.get(), 1);
    assert!(
        token.iter().all(|byte| *byte == 0x5a),
        "entropy bytes were changed"
    );
}

#[test]
fn entropy_failure_after_a_partial_fill_never_returns_a_partial_token() {
    let result = generate_with(|destination| {
        assert_eq!(destination.len(), 32);
        destination[..16].fill(0x5a);
        Err(getrandom::Error::UNSUPPORTED)
    });
    assert!(
        result.is_err(),
        "failed entropy source returned token bytes"
    );
    let message = result.err().unwrap().to_string();
    assert!(!message.contains("5a5a") && !message.contains("ZZZZ"));
}
