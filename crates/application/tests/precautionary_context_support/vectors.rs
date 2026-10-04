use application::case_stages::*;
use application::cases::*;
use application::precautionary_hearings::{PrecautionaryContext, PrecautionaryContextMaterial};
use domain::cases::{CaseId, CaseMetadata};
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::identity::UserId;
use time::{Date, Month, OffsetDateTime, UtcOffset};

use crate::precautionary_context_support::Hasher;

// Independently packed PCTX1 vectors: raw UUIDs, big-endian counters/timestamps,
// u32-length-prefixed strings and blobs, and explicit option/enum tags. Nested
// CADM1/CSTG1 digests use Hasher's deterministic test algorithm, not real SHA-256.
const INITIAL_HEX: &str = concat!(
    "504354583100000000000000000000000000000001000000000000000000000000000000010000000100000030434144",
    "4d31000000000141000000014201000000014e0000000150000000014a000000014300000001000000014f0000636368",
    "749c2a2c2e3133753639893d801111121315631617181a6a1b1c1d1f6900000000000000000000000700000000000000",
    "0000000000000000000000000200000001780000000000000000000000000000000100000001000000304341444d3100",
    "0000000141000000014201000000014e0000000150000000014a000000014300000001000000014f0000636368749c2a",
    "2c2e3133753639893d801111121315631617181a6a1b1c1d1f6900000000000000000000000700000000000000000000",
    "0000000000000000000200000001780000000000000000000000000000000001000000010000000001636368749c2a2c",
    "2e3133753639893d801111121315631617181a6a1b1c1d1f690000000000000000000000070000000000000000000000",
    "0000000000000000020000000178",
);

const CHANGED_HEX: &str = concat!(
    "504354583100000000000000000000000000000001000000000000000000000000000000010000000200000030434144",
    "4d31010000000141000000014201000000014e0000000150000000014a000000014300000001000000014f0000636368",
    "749c2b2c2e3133753639893d801111121315631617181a6a1b1c1d1f6900000000000000020000000300000000000000",
    "00000000000000000000000005000000056c617465720000000000000000000000000000000100000001000000304341",
    "444d31000000000141000000014201000000014e0000000150000000014a000000014300000001000000014f00006363",
    "68749c2a2c2e3133753639893d801111121315631617181a6a1b1c1d1f69000000000000000000000007000000000000",
    "000000000000000000000000000200000001780100000000000000000000000000000001000000020200000001636368",
    "749c2a2c2e3133753639893d801111121315631617181a6a1b1c1d1f6900000000000000010000000200000000000000",
    "0000000000000000000000000600000005737461676501010000009a4353544731020007b20101000000000000000000",
    "000000000000000000000300000001030303030303030303030303030303030303030303030303030303030303030301",
    "00000000000000000000000900000e100000000143010000000152010000000000000000000000000000000400000002",
    "040404040404040404040404040404040404040404040404040404040404040401000000014e8a9fa69e8c6265712175",
    "7a7e83888d9f979cafb8acb0b5bb03132f33373c94440000000200000000000000000000000000000003000000010303",
    "03030303030303030303030303030303030303030303030303030303030300000005642e706466000000000000000000",
    "000000000000000004000000020404040404040404040404040404040404040404040404040404040404040404000000",
    "06722e646f63780100",
);

fn actor(id: u128, email: &str) -> CaseActorSnapshot {
    CaseActorSnapshot {
        id: UserId::from_uuid(uuid::Uuid::from_u128(id)),
        email: email.into(),
    }
}

fn initial_material() -> PrecautionaryContextMaterial {
    let case_id = CaseId::from_uuid(uuid::Uuid::from_u128(1));
    let values = PenalCaseCreation::new(
        CaseMetadata::new("A", "B").unwrap(),
        PenalCaseProfile::new("N", "P", "J", "C", &["O"], None, None).unwrap(),
    )
    .into_values();
    let administration = CaseAdministrationSnapshot {
        case_id,
        revision: CaseRevision::FIRST,
        values_digest: case_administration_digest(&Hasher, &values),
        values,
        changed_at: OffsetDateTime::from_unix_timestamp_nanos(7).unwrap(),
        changed_by: actor(2, "x"),
    };
    PrecautionaryContextMaterial {
        case_id,
        stage: CaseStageEntry::Initial(CaseInitialStageRegistration {
            case_id,
            stage_revision: CaseStageRevision::FIRST,
            administration_revision: CaseRevision::FIRST,
            stage: InitialCaseStage::Investigation,
            administration_digest: administration.values_digest,
            recorded_at: administration.changed_at,
            recorded_by: administration.changed_by.clone(),
        }),
        stage_administration: administration.clone(),
        administration,
    }
}

fn support(id: u128, version: u32) -> StageSupportRef {
    StageSupportRef::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(uuid::Uuid::from_u128(id)),
            version: DocumentVersion::new(version).unwrap(),
        },
        Sha256Digest::from_array([id as u8; 32]),
    )
}

fn changed_material() -> PrecautionaryContextMaterial {
    let mut material = initial_material();
    let order = support(3, 1);
    let receipt = support(4, 2);
    let values = CaseStageChange::Transition(
        StageTransition::to_trial(
            DeclaredStageTime::date(
                Date::from_calendar_date(1970, Month::January, 1).unwrap(),
                UtcOffset::UTC,
            )
            .unwrap(),
            order,
            DeclaredStageTime::instant(
                OffsetDateTime::from_unix_timestamp_nanos(9)
                    .unwrap()
                    .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
            )
            .unwrap(),
            StageCourt::new("C").unwrap(),
            Some(StageReceiptReference::new("R").unwrap()),
            Some(receipt),
            Some(StageNote::new("N").unwrap()),
        )
        .unwrap(),
    );
    material.stage = CaseStageEntry::Changed(Box::new(CaseStageSnapshot {
        case_id: material.case_id,
        stage_revision: CaseStageRevision::new(2).unwrap(),
        from_stage: Some(CaseStage::Intermediate),
        values_digest: case_stage_digest(&Hasher, &values),
        values,
        administration_revision: material.stage_administration.revision,
        administration_digest: material.stage_administration.values_digest,
        supports: vec![
            StageSupportSnapshot {
                reference: order.reference(),
                digest: order.digest(),
                name: "d.pdf".into(),
                format: StageDocumentFormat::Pdf,
                policy: StageFormatPolicy::PdfDocxV1,
            },
            StageSupportSnapshot {
                reference: receipt.reference(),
                digest: receipt.digest(),
                name: "r.docx".into(),
                format: StageDocumentFormat::Docx,
                policy: StageFormatPolicy::PdfDocxV1,
            },
        ],
        recorded_at: OffsetDateTime::from_unix_timestamp_nanos(1_000_000_002).unwrap(),
        recorded_by: actor(6, "stage"),
    }));
    material.administration.revision = CaseRevision::new(2).unwrap();
    material.administration.values = material
        .administration
        .values
        .with_status(CaseAdministrativeStatus::Closed);
    material.administration.values_digest =
        case_administration_digest(&Hasher, &material.administration.values);
    material.administration.changed_at =
        OffsetDateTime::from_unix_timestamp_nanos(2_000_000_003).unwrap();
    material.administration.changed_by = actor(5, "later");
    material
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn initial_context_matches_independent_pctx1_golden_vector() {
    let context = PrecautionaryContext::new(&Hasher, initial_material()).unwrap();
    assert_eq!(hex(&context.canonical_bytes()), INITIAL_HEX);
    assert_eq!(
        hex(context.digest(&Hasher).as_bytes()),
        "b254d9f11a697c6bf27e9258334679eebbf5dfe34acc25c539944908a676f796"
    );
}

#[test]
fn changed_context_matches_independent_pctx1_golden_vector() {
    let context = PrecautionaryContext::new(&Hasher, changed_material()).unwrap();
    assert_eq!(hex(&context.canonical_bytes()), CHANGED_HEX);
    assert_eq!(
        hex(context.digest(&Hasher).as_bytes()),
        "8b0779f43d9337371ca8033103f91a4d96ec2c4e0bc990b4944939236c9fc021"
    );
}
