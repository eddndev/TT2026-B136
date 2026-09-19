#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
use deadline_observation_support::*;

use application::{
    deadline_inputs::DeadlineSourceDetail, deadline_profiles::*, procedural_facts::*,
};
use domain::{cases::CaseId, crypto::Sha256Digest, judicial_calendars::*};
use uuid::Uuid;

#[test]
fn corrupt_or_cross_case_profile_cannot_hide_behind_unknown_source() {
    for kind in 0..3 {
        let (mut profile, _) = fixture();
        let (_, material) = inputs::unknown();
        if kind == 0 {
            profile.receipt.submission_digest = Sha256Digest::from_array([0; 32]);
        }
        if kind == 1 {
            profile.definition_digest = Sha256Digest::from_array([0; 32]);
        }
        if kind == 2 {
            let mut input = profile_input(&profile);
            input.scope = DeadlineProfileScope::Case(CaseId::from_uuid(Uuid::nil()));
            profile.definition = DeadlineProfileDefinition::new(input).unwrap();
            resign_profile(&mut profile);
        }
        assert!(build(&profile, &material, None).is_err());
    }
}

#[test]
fn selected_and_head_presence_must_match_for_sources_and_calendars() {
    for slot in 0..4 {
        let (profile, mut material) = fixture();
        calendar_pair(&mut material);
        match slot {
            0 => material.source = None,
            1 => material.source_head = None,
            2 => material.calendar = None,
            3 => material.calendar_head = None,
            _ => unreachable!(),
        }
        assert!(build(&profile, &material, None).is_err());
    }
}

#[test]
fn both_selected_and_head_fact_receipts_are_verified() {
    for selected in [false, true] {
        for kind in 0..3 {
            let (profile, mut material) = fixture();
            material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
                2,
                false,
                "2026-01-07",
            ))));
            let source = fact_mut(if selected {
                &mut material.source
            } else {
                &mut material.source_head
            });
            let metadata = inputs::metadata_mut(source);
            match kind {
                0 => metadata.values_digest = Sha256Digest::from_array([0; 32]),
                1 => metadata.receipt.submission_digest = Sha256Digest::from_array([0; 32]),
                2 => metadata.receipt.sources_digest = Sha256Digest::from_array([0; 32]),
                _ => unreachable!(),
            }
            assert!(build(&profile, &material, None).is_err());
        }
    }
}

#[test]
fn both_calendar_receipts_and_values_are_verified_without_source() {
    for selected in [false, true] {
        for values in [false, true] {
            let (profile, _) = fixture();
            let (_, mut material) = inputs::unknown();
            calendar_pair(&mut material);
            let calendar = if selected {
                &mut material.calendar
            } else {
                &mut material.calendar_head
            }
            .as_mut()
            .unwrap();
            if values {
                calendar.values_digest = Sha256Digest::from_array([0; 32]);
            } else {
                calendar.receipt.submission_digest = Sha256Digest::from_array([0; 32]);
            }
            assert!(build(&profile, &material, None).is_err());
        }
    }
}

#[test]
fn source_heads_cannot_change_family_identity_case_or_regress() {
    for kind in 0..6 {
        let (profile, mut material) = fixture();
        material.source = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
            2,
            false,
            "2026-01-07",
        ))));
        material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
            3,
            false,
            "2026-01-08",
        ))));
        match kind {
            0 => {
                material.source_head = Some(DeadlineSourceDetail::HearingResult(Box::new(
                    inputs::hearing(3, false, "2026-01-08", &[]),
                )))
            }
            1 => {
                material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(
                    inputs::notification(3, false, 1, "2026-01-08"),
                )))
            }
            2 | 3 => {
                let head = fact_mut(&mut material.source_head);
                let ProceduralFactSnapshot::Resolution(snapshot) = &mut head.snapshot else {
                    unreachable!()
                };
                snapshot.root = ResolutionRoot::new(
                    if kind == 2 {
                        ResolutionId::from_uuid(Uuid::nil())
                    } else {
                        snapshot.root.id()
                    },
                    if kind == 3 {
                        CaseId::from_uuid(Uuid::nil())
                    } else {
                        snapshot.root.case_id()
                    },
                );
                inputs::resign_fact(head);
            }
            4 => {
                material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(
                    inputs::resolution(1, false, "2026-01-06"),
                )))
            }
            5 => {
                for item in [&mut material.source, &mut material.source_head] {
                    let detail = fact_mut(item);
                    let ProceduralFactSnapshot::Resolution(snapshot) = &mut detail.snapshot else {
                        unreachable!()
                    };
                    snapshot.root =
                        ResolutionRoot::new(snapshot.root.id(), CaseId::from_uuid(Uuid::nil()));
                    inputs::resign_fact(detail);
                }
            }
            _ => unreachable!(),
        }
        assert!(build(&profile, &material, None).is_err());
    }
}

#[test]
fn the_same_source_revision_cannot_substitute_uncommitted_readable_metadata() {
    let (profile, mut material) = fixture();
    let head = fact_mut(&mut material.source_head);
    inputs::metadata_mut(head).recorded_by.email = "different@example.test".into();
    fact_receipt_matches(inputs::hasher().as_ref(), head).unwrap();
    assert!(build(&profile, &material, None).is_err());
}

#[test]
fn calendar_heads_keep_identity_scope_revision_and_exact_metadata() {
    for kind in 0..4 {
        let (profile, mut material) = fixture();
        calendar_pair(&mut material);
        if kind == 0 {
            material.calendar.as_mut().unwrap().id = JudicialCalendarId::from_uuid(Uuid::nil());
            inputs::resign_calendar(material.calendar.as_mut().unwrap());
        } else if kind == 1 {
            let scope = inputs::calendar(1, false, JudicialCalendarClassification::Countable)
                .values
                .scope()
                .clone();
            let scope = JudicialCalendarScope::new(JudicialCalendarScopeInput {
                title: "Different scope",
                jurisdiction: scope.jurisdiction(),
                entity_codes: &["09"],
                authority: scope.authority(),
                organ: scope.organ(),
                territory: scope.territory(),
                use_description: scope.use_description(),
            })
            .unwrap();
            let head = material.calendar_head.as_mut().unwrap();
            let v = &head.values;
            head.values = JudicialCalendarValues::new(
                scope,
                v.coverage(),
                v.sources().to_vec(),
                v.weekly_pattern().to_vec(),
                v.exceptions().to_vec(),
            )
            .unwrap();
            inputs::resign_calendar(head);
        } else if kind == 2 {
            std::mem::swap(&mut material.calendar, &mut material.calendar_head);
        } else {
            material.calendar_head = material.calendar.clone();
            material.calendar_head.as_mut().unwrap().recorded_by.email =
                "different@example.test".into();
        }
        assert!(build(&profile, &material, None).is_err());
    }
}

#[test]
fn observed_notification_parent_is_required_and_must_be_an_exact_related_resolution() {
    let (profile, ordinary) = fixture();
    let (material, original_parent) = notification_material();
    assert!(build(&profile, &material, None).is_err());
    assert!(build(&profile, &ordinary, Some(&original_parent)).is_err());
    for kind in 0..4 {
        let mut parent = original_parent.clone();
        if kind == 0 {
            parent = inputs::notification(3, false, 1, "2026-01-08");
        } else if kind == 1 {
            inputs::metadata_mut(&mut parent).receipt.submission_digest =
                Sha256Digest::from_array([0; 32]);
        } else {
            let ProceduralFactSnapshot::Resolution(snapshot) = &mut parent.snapshot else {
                unreachable!()
            };
            snapshot.root = ResolutionRoot::new(
                if kind == 2 {
                    ResolutionId::from_uuid(Uuid::nil())
                } else {
                    snapshot.root.id()
                },
                if kind == 3 {
                    CaseId::from_uuid(Uuid::nil())
                } else {
                    snapshot.root.case_id()
                },
            );
            inputs::resign_fact(&mut parent);
        }
        assert!(build(&profile, &material, Some(&parent)).is_err());
    }
}

#[test]
fn equal_parent_revision_requires_matching_committed_projection_and_readable_view() {
    let (profile, _) = fixture();
    let parent = inputs::resolution(2, false, "2026-01-01");
    for selected in [false, true] {
        let (mut original, _) = notification_material();
        if selected {
            original.source = Some(DeadlineSourceDetail::Fact(Box::new(inputs::notification(
                1,
                false,
                2,
                "2026-01-06",
            ))));
            original.source_head = Some(DeadlineSourceDetail::Fact(Box::new(
                inputs::notification(2, false, 1, "2026-01-07"),
            )));
        }
        assert!(build(&profile, &original, Some(&parent)).is_ok());
        for view in [false, true] {
            let mut material = original.clone();
            let detail = fact_mut(if selected {
                &mut material.source
            } else {
                &mut material.source_head
            });
            if view {
                detail.sources.views.resolution.as_mut().unwrap().summary =
                    FactText::new("Substituted projection").unwrap();
            } else {
                detail
                    .sources
                    .resolved
                    .resolution
                    .as_mut()
                    .unwrap()
                    .submission_digest = Sha256Digest::from_array([222; 32]);
            }
            inputs::resign_fact(detail);
            fact_receipt_matches(inputs::hasher().as_ref(), detail).unwrap();
            assert!(build(&profile, &material, Some(&parent)).is_err());
        }
    }
}

#[test]
fn both_result_receipts_and_hearing_identity_are_verified() {
    for kind in 0..6 {
        let (profile, _) = fixture();
        let selected = inputs::hearing(1, false, "2026-01-06", &[]);
        let mut material =
            inputs::material(DeadlineSourceDetail::HearingResult(Box::new(selected)));
        material.source_head = Some(DeadlineSourceDetail::HearingResult(Box::new(
            inputs::hearing(2, false, "2026-01-07", &[]),
        )));
        if kind == 4 {
            material.source_head = material.source.clone();
        }
        let source = if kind == 0 {
            &mut material.source
        } else {
            &mut material.source_head
        };
        let Some(DeadlineSourceDetail::HearingResult(detail)) = source else {
            unreachable!()
        };
        match kind {
            0 | 1 => detail.snapshot.receipt.submission_digest = Sha256Digest::from_array([0; 32]),
            2 => {
                detail.snapshot.hearing_id = domain::hearings::HearingId::from_uuid(Uuid::nil());
                detail.snapshot.anchor.hearing_id = detail.snapshot.hearing_id;
                detail.anchor.reference = detail.snapshot.anchor;
                inputs::resign_hearing(detail);
            }
            3 => {
                detail.snapshot.case_id = CaseId::from_uuid(Uuid::nil());
                inputs::resign_hearing(detail);
            }
            4 => detail.snapshot.recorded_by.email = "Changed metadata".into(),
            5 => {
                detail.snapshot.id =
                    domain::hearing_results::HearingResultId::from_uuid(Uuid::nil());
                inputs::resign_hearing(detail);
            }
            _ => unreachable!(),
        }
        assert!(build(&profile, &material, None).is_err());
    }
}
