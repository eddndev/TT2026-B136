//! Behavior of the audit log adapters against the real SHA-256 hasher:
//! persistence across adapter instances, corruption detection, and
//! property-based checks of the chain.

use domain::audit::{verify_chain, AuditLog, ChainVerification};
use domain::crypto::Sha256Digest;
use infrastructure::{FileAuditLog, InMemoryAuditLog, RingSha256Hasher};
use proptest::prelude::*;
use time::OffsetDateTime;

fn timestamp(offset_seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_735_689_600 + offset_seconds).unwrap()
}

#[test]
fn appends_from_separate_adapter_instances_keep_one_valid_chain() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.jsonl");

    // Two adapter instances simulate two separate process runs over the
    // same file.
    {
        let mut first_run = FileAuditLog::new(&path, RingSha256Hasher::new());
        first_run
            .append("ana", "open", "case-1", timestamp(0))
            .unwrap();
        first_run
            .append("ana", "edit", "case-1", timestamp(1))
            .unwrap();
    }
    let mut second_run = FileAuditLog::new(&path, RingSha256Hasher::new());
    second_run
        .append("bob", "close", "case-1", timestamp(2))
        .unwrap();

    let entries = second_run.load_all().unwrap();
    let sequences: Vec<u64> = entries.iter().map(|e| e.event.sequence).collect();
    assert_eq!(sequences, vec![0, 1, 2]);
    assert_eq!(
        verify_chain(&RingSha256Hasher::new(), &entries).unwrap(),
        ChainVerification::Valid { entries: 3 }
    );
}

#[test]
fn concurrent_appends_serialize_into_one_valid_chain() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.jsonl");

    // Several adapter instances append to the same file at the same time,
    // as concurrent process runs would. Each append must observe the entry
    // a racing writer just added, so the sequence numbers stay unique and
    // every entry links to the actual previous chain value.
    const WRITERS: usize = 8;
    const APPENDS_PER_WRITER: usize = 4;
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(WRITERS));
    let handles: Vec<_> = (0..WRITERS)
        .map(|writer| {
            let path = path.clone();
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                let mut log = FileAuditLog::new(&path, RingSha256Hasher::new());
                barrier.wait();
                for round in 0..APPENDS_PER_WRITER {
                    log.append(
                        "ana",
                        "open",
                        &format!("case-{writer}-{round}"),
                        timestamp((writer * APPENDS_PER_WRITER + round) as i64),
                    )
                    .unwrap();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }

    let entries = FileAuditLog::new(&path, RingSha256Hasher::new())
        .load_all()
        .unwrap();
    let total = WRITERS * APPENDS_PER_WRITER;
    let sequences: Vec<u64> = entries.iter().map(|e| e.event.sequence).collect();
    let expected: Vec<u64> = (0..total as u64).collect();
    assert_eq!(sequences, expected);
    assert_eq!(
        verify_chain(&RingSha256Hasher::new(), &entries).unwrap(),
        ChainVerification::Valid { entries: total }
    );
}

#[test]
fn a_hand_corrupted_line_breaks_verification_at_its_index() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.jsonl");
    let mut log = FileAuditLog::new(&path, RingSha256Hasher::new());
    for i in 0..3 {
        log.append("ana", "open", "case-1", timestamp(i)).unwrap();
    }

    // Change the actor of the second line without touching its chain value.
    let content = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<String> = content
        .lines()
        .enumerate()
        .map(|(i, line)| {
            if i == 1 {
                line.replace("\"actor\":\"ana\"", "\"actor\":\"mallory\"")
            } else {
                line.to_string()
            }
        })
        .collect();
    assert_ne!(lines.join("\n"), content.trim_end());
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();

    let entries = FileAuditLog::new(&path, RingSha256Hasher::new())
        .load_all()
        .unwrap();
    assert_eq!(
        verify_chain(&RingSha256Hasher::new(), &entries).unwrap(),
        ChainVerification::Broken {
            first_broken_index: 1,
        }
    );
}

/// One event's worth of test data.
#[derive(Clone, Debug)]
struct EventData {
    actor: String,
    action: String,
    resource: String,
    at_offset: i64,
}

fn event_data() -> impl Strategy<Value = EventData> {
    (
        "[a-z]{1,10}",
        "[a-z]{1,10}",
        "[a-z0-9-]{1,12}",
        0i64..=3_000_000,
    )
        .prop_map(|(actor, action, resource, at_offset)| EventData {
            actor,
            action,
            resource,
            at_offset,
        })
}

fn filled_log(events: &[EventData]) -> InMemoryAuditLog<RingSha256Hasher> {
    let mut log = InMemoryAuditLog::new(RingSha256Hasher::new());
    for data in events {
        log.append(
            &data.actor,
            &data.action,
            &data.resource,
            timestamp(data.at_offset),
        )
        .unwrap();
    }
    log
}

/// Flips the lowest bit of one byte of an ASCII string, or appends a byte
/// when the string is empty, so the field always changes but stays UTF-8.
fn corrupt_ascii(text: &str, position: usize) -> String {
    let mut bytes = text.as_bytes().to_vec();
    if bytes.is_empty() {
        bytes.push(b'x');
    } else {
        let at = position % bytes.len();
        bytes[at] ^= 0x01;
    }
    String::from_utf8(bytes).expect("ascii stays utf-8 after flipping the low bit")
}

proptest! {
    #[test]
    fn random_event_sequences_verify_ok(events in proptest::collection::vec(event_data(), 0..12)) {
        let log = filled_log(&events);
        let entries = log.load_all().unwrap();
        prop_assert_eq!(
            verify_chain(&RingSha256Hasher::new(), &entries).unwrap(),
            ChainVerification::Valid { entries: events.len() }
        );
    }

    #[test]
    fn any_single_corruption_is_detected_at_its_index(
        events in proptest::collection::vec(event_data(), 1..10),
        target in any::<proptest::sample::Index>(),
        field in 0usize..5,
        byte in any::<proptest::sample::Index>(),
        bit in 0u32..8,
    ) {
        let log = filled_log(&events);
        let mut entries = log.load_all().unwrap();
        let index = target.index(entries.len());
        let entry = &mut entries[index];
        match field {
            0 => {
                // Flip one bit of one byte of the stored chain value.
                let mut bytes = *entry.chain.as_bytes();
                bytes[byte.index(bytes.len())] ^= 1 << bit;
                entry.chain = Sha256Digest::from_array(bytes);
            }
            1 => entry.event.sequence ^= 1 << (bit + 8),
            2 => entry.event.actor = corrupt_ascii(&entry.event.actor, byte.index(64)),
            3 => entry.event.action = corrupt_ascii(&entry.event.action, byte.index(64)),
            _ => entry.event.resource = corrupt_ascii(&entry.event.resource, byte.index(64)),
        }
        prop_assert_eq!(
            verify_chain(&RingSha256Hasher::new(), &entries).unwrap(),
            ChainVerification::Broken { first_broken_index: index }
        );
    }
}
