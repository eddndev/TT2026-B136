use super::*;
use domain::{
    crypto::Sha256Digest,
    hearings::{HearingId, HearingRevision},
    precautionary_hearings::{
        PrecautionaryHearingId, PrecautionaryHearingRevision, PrecautionaryMeasureRef,
    },
};

#[test]
fn predecessor_effects_reject_wrong_capture_digest_without_partial_group_or_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let captured = &first.group.measures[0];
    let mut digest = *captured.capture_digest.as_bytes();
    digest[0] ^= 1;
    let previous = PrecautionaryMeasureRef::new(
        captured.result.id,
        captured.result.revision,
        Sha256Digest::from_array(digest),
    );
    for effect in [
        MeasureEffect::Confirm { previous },
        MeasureEffect::Modify {
            previous,
            values: values(&seed.subject),
        },
        MeasureEffect::Revoke { previous },
        MeasureEffect::Cease { previous },
        MeasureEffect::Substitute {
            predecessors: vec![previous],
            successors: vec![MeasureProposal {
                id: MeasureId::new(),
                values: values(&seed.subject),
            }],
        },
    ] {
        let mut command = fresh(&seed.command);
        command.outcome =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![effect]))
                .unwrap();
        let before = snapshot(&mut db);
        assert!(service(&db, seed.actor.clone())
            .prepare("session", db.case, command)
            .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn unsupported_anchor_families_reject_without_synthetic_receipts() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    for anchor in [
        MeasureDecisionAnchorRef::Initial {
            hearing_id: HearingId::new(),
            revision: HearingRevision::new(1).unwrap(),
            values_digest: Sha256Digest::from_array([1; 32]),
            submission_digest: Sha256Digest::from_array([2; 32]),
        },
        MeasureDecisionAnchorRef::Precautionary {
            hearing_id: PrecautionaryHearingId::new(),
            revision: PrecautionaryHearingRevision::new(1).unwrap(),
            capture_digest: Sha256Digest::from_array([3; 32]),
        },
    ] {
        let mut command = no_change(&seed.command);
        command.anchor = Some(anchor);
        let before = snapshot(&mut db);
        assert!(service(&db, seed.actor.clone())
            .prepare("session", db.case, command)
            .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}
