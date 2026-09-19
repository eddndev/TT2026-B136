mod procedural_fact_support;
mod procedural_resource_support;
use domain::{
    procedural_facts::*, procedural_resources::*, procedural_time::DeclaredProceduralTime,
};
use procedural_fact_support::{evidence, label, text};
use procedural_resource_support::*;
use time::UtcOffset;
use uuid::Uuid;

fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn fixed_vectors_bind_versioned_resource_and_act_declarations_independently() {
    let resource = ResourceValues::new(ResourceValuesInput {
        kind: ResourceKind::Revocation,
        mode: FactDeclaration::Known(ResourceMode::Written),
        title: label("t"),
        resolution: FactResolutionRef {
            id: ResolutionId::from_uuid(Uuid::from_u128(1)),
            revision: FactRevision::initial(),
        },
        resolution_evidence: evidence(2, 1, 3, "l"),
        resolution_reference: FactDeclaration::Unknown(text("r")),
        issuing_authority: FactDeclaration::Unknown(text("i")),
        receiving_authority: None,
        resolution_at: DeclaredProceduralTime::unknown(),
        notification_at: None,
        challenged_part: text("p"),
        grounds: text("g"),
        appellants: vec![ResourceAppellant::new(
            label("n"),
            FactDeclaration::Unknown(text("c")),
            None,
        )],
    })
    .unwrap();
    let expected = concat!(
        "5052534331",
        "000101",
        "0000000174",
        "00000000000000000000000000000001",
        "00000001",
        "00000000000000000000000000000002",
        "00000001",
        "0303030303030303030303030303030303030303030303030303030303030303",
        "000000016c",
        "000000000172",
        "000000000169",
        "000000",
        "0000000170",
        "0000000167",
        "01",
        "000000016e",
        "000000000163",
        "00"
    );
    assert_eq!(resource.canonical_bytes(), hex(expected));
    let act = ResourceActValues::new(ResourceActValuesInput {
        kind: ResourceActKind::Interposition,
        mode: FactDeclaration::Known(ResourceMode::Written),
        occurred_at: DeclaredProceduralTime::unknown(),
        authority: FactDeclaration::Unknown(text("a")),
        statement: text("s"),
        evidence: vec![evidence(2, 1, 3, "l")],
    })
    .unwrap();
    let expected = concat!(
        "5052414331",
        "00010100",
        "000000000161",
        "0000000173",
        "01",
        "00000000000000000000000000000002",
        "00000001",
        "0303030303030303030303030303030303030303030303030303030303030303",
        "000000016c"
    );
    assert_eq!(act.canonical_bytes(), hex(expected));
    assert_ne!(resource.canonical_bytes(), act.canonical_bytes());
}

#[test]
fn absent_unknown_and_precise_times_and_receiving_authorities_remain_distinct() {
    let date = "2026-09-19".parse().unwrap();
    let times = [
        None,
        Some(DeclaredProceduralTime::unknown()),
        Some(DeclaredProceduralTime::date(date, None).unwrap()),
        Some(DeclaredProceduralTime::date(date, Some(UtcOffset::UTC)).unwrap()),
        Some(DeclaredProceduralTime::minute(date, 0, 0, None).unwrap()),
        Some(DeclaredProceduralTime::second(date, 0, 0, 0, None).unwrap()),
        Some(DeclaredProceduralTime::second(date, 0, 0, 0, Some(UtcOffset::UTC)).unwrap()),
    ];
    let encodings: Vec<_> = times
        .into_iter()
        .map(|at| {
            let mut value = input();
            value.notification_at = at;
            ResourceValues::new(value).unwrap().canonical_bytes()
        })
        .collect();
    for (index, first) in encodings.iter().enumerate() {
        for next in encodings.iter().skip(index + 1) {
            assert_ne!(first, next);
        }
    }
    let mut value = input();
    let absent = ResourceValues::new(value.clone())
        .unwrap()
        .canonical_bytes();
    value.receiving_authority = Some(FactDeclaration::Unknown(text("unknown")));
    let unknown = ResourceValues::new(value.clone())
        .unwrap()
        .canonical_bytes();
    value.receiving_authority = Some(FactDeclaration::Known(label("unknown")));
    let known = ResourceValues::new(value).unwrap().canonical_bytes();
    assert_ne!(absent, unknown);
    assert_ne!(unknown, known);
    assert_ne!(absent, known);
}

#[test]
fn equal_instants_do_not_erase_declared_precision_or_original_offset() {
    let date = "2026-09-19".parse().unwrap();
    let mut first = act_input();
    first.occurred_at =
        DeclaredProceduralTime::second(date, 12, 0, 0, Some(UtcOffset::UTC)).unwrap();
    let mut second = first.clone();
    second.occurred_at =
        DeclaredProceduralTime::second(date, 6, 0, 0, Some(UtcOffset::from_hms(-6, 0, 0).unwrap()))
            .unwrap();
    assert_eq!(
        first.occurred_at.instant_value(),
        second.occurred_at.instant_value()
    );
    assert_ne!(
        ResourceActValues::new(first).unwrap().canonical_bytes(),
        ResourceActValues::new(second).unwrap().canonical_bytes()
    );
}
