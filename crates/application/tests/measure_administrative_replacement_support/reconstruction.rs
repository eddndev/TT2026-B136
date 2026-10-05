use crate::replacement_support::*;
use domain::crypto::{DocumentHasher, Sha256Digest};

fn rehash(capture: &mut MeasureAdministrativeCapture) -> bool {
    let Ok(review) = measure_administrative_review_bytes(&capture.review) else {
        return false;
    };
    capture.review.review_digest = Hasher.hash_bytes(&review);
    for row in &mut capture.records {
        row.review_digest = capture.review.review_digest;
        let Ok(bytes) = measure_administrative_record_bytes(row) else {
            return false;
        };
        row.capture_digest = Hasher.hash_bytes(&bytes);
    }
    let Ok(bytes) = measure_administrative_capture_bytes(capture) else {
        return false;
    };
    capture.capture_digest = Hasher.hash_bytes(&bytes);
    true
}

#[test]
fn full_reconstruction_rejects_missing_extra_reordered_or_rehashed_joint_rows_and_links() {
    let fixture = ReplacementFixture::initial();
    let original = fixture.capture();
    for mutation in 0..7 {
        let mut changed = original.clone();
        match mutation {
            0 => {
                changed.records.pop();
            }
            1 => changed.records.push(changed.records[0].clone()),
            2 => changed.records.reverse(),
            3 => changed.replacement_link = None,
            4 => {
                let link = changed.replacement_link.as_mut().unwrap();
                std::mem::swap(&mut link.entered_in_error, &mut link.replacement);
            }
            5 => changed.records[0].actor.email = "invented-source-author@example.test".into(),
            _ => {
                changed.records[0].result.previous = PrecautionaryMeasureRef::new(
                    fixture.command.target.id(),
                    fixture.command.target.revision(),
                    Sha256Digest::from_array([9; 32]),
                )
            }
        }
        if rehash(&mut changed) {
            assert!(
                measure_administrative_capture_with_decision_history_matches(
                    &Hasher,
                    &changed,
                    &fixture.history,
                )
                .is_err(),
                "accepted joint mutation {mutation}"
            );
        }
    }
}

#[test]
fn coherently_rehashed_replacement_cannot_change_other_terms_or_the_retained_last_judicial_source()
{
    let fixture = ReplacementFixture::initial();
    let original = fixture.capture();
    for mutation in 0..4 {
        let mut changed = original.clone();
        let result = changed.review.replacement.as_mut().unwrap();
        match mutation {
            0 => {
                result.values = MeasureValues::new(MeasureValuesInput {
                    subject: result.values.subject(),
                    kind: result.values.kind(),
                    conditions: note("Invented replacement terms"),
                    validity: result.values.validity().clone(),
                    supervision: result.values.supervision().clone(),
                })
            }
            1 => result.last_action = MeasureCaptureAction::Cease,
            2 => result.record_root = MeasureRecordRoot::Judicial(result.judicial_origin),
            _ => result.last_judicial.owner.group_digest = Sha256Digest::from_array([7; 32]),
        }
        let id = result.id;
        changed
            .records
            .iter_mut()
            .find(|row| row.result.id == id)
            .unwrap()
            .result = result.clone();
        assert!(rehash(&mut changed));
        let marked = record_reference(row(&changed, changed.review.result.id));
        let replacement = record_reference(row(&changed, id));
        changed.replacement_link = Some(MeasureAdministrativeReplacementLink {
            entered_in_error: marked,
            replacement,
        });
        changed.capture_digest =
            Hasher.hash_bytes(&measure_administrative_capture_bytes(&changed).unwrap());
        assert!(
            measure_administrative_capture_with_decision_history_matches(
                &Hasher,
                &changed,
                &fixture.history,
            )
            .is_err()
        );
    }
}
