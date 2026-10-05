use super::*;

#[test]
fn frame_prefix_truncation_mutation_trailing_bytes_and_global_sizes_are_strict() {
    for (stored, maximum) in [
        (stored_decision(unknown_decision()), 16_076),
        (stored_measure(unknown_measure()), 20_113),
        (stored_outcome(mixed()), 645_450),
    ] {
        for size in [0, 4, stored.bytes.len() - 1] {
            stored.rejects_bytes(&stored.bytes[..size], &stored.view);
        }
        for index in [0, stored.bytes.len() - 1] {
            let mut bytes = stored.bytes.clone();
            bytes[index] ^= 1;
            stored.rejects_bytes(&bytes, &stored.view);
        }
        let mut bytes = stored.bytes.clone();
        bytes.push(0);
        stored.rejects_bytes(&bytes, &stored.view);
        bytes.resize(maximum + 1, 0);
        stored.rejects_bytes(&bytes, &Value::Null);
    }
}
